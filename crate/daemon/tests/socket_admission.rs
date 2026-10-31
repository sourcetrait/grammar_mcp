//! The engine socket: a same-UID stream answered, each malformed one closed alone.

mod support;

use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use sourcetrait_grammar_protocol::{
    ClientInstanceId, Expectation, HELLO_BUDGET, HELLO_FRAME_MAX, HelloReply, Instance, LIMITS_DEFAULT,
    LeaseIntent, PREFACE, Session, StreamRole, check_reply, decode_payload, framed, payload,
};
use support::{TestInstance, start, start_with};

fn uid() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// A Status stream's hello to `instance`'s worker at `socket`, its reply checked.
async fn hello(
    socket: &Path,
    instance: Instance,
) -> Result<Session, String> {
    let stream = tokio::net::UnixStream::connect(socket).await.map_err(|e| e.to_string())?;
    let mut framed = framed(stream);
    let expect = Expectation {
        uid: uid(),
        instance,
        role: StreamRole::Status,
        lease: LeaseIntent::Observe,
        requested: LIMITS_DEFAULT,
        required: Vec::new(),
        last: None,
    };
    framed.send(payload(&expect.hello(ClientInstanceId([7; 16])))).await.map_err(|e| e.to_string())?;
    let frame = framed.next().await.ok_or("the worker closed the stream")?.map_err(|e| e.to_string())?;
    let reply: HelloReply = decode_payload(&frame, HELLO_BUDGET).map_err(|e| e.to_string())?;
    check_reply(reply, &expect).map_err(|e| format!("{e:?}"))
}

/// Whether the worker closes `stream` within `within`, writing nothing back.
///
/// macOS refuses a read timeout on a socket its peer has already closed, so a
/// refused timeout is answered by a read that doesn't wait; a reset is a close.
fn closed_within(
    stream: &mut UnixStream,
    within: Duration,
) -> bool {
    let mut buf = [0u8; 64];
    if stream.set_read_timeout(Some(within)).is_err() {
        stream.set_nonblocking(true).unwrap();
    }
    match stream.read(&mut buf) {
        Ok(0) => true,
        Ok(_) => false,
        Err(e) => e.kind() == std::io::ErrorKind::ConnectionReset,
    }
}

fn default_instance() -> Instance {
    Instance::new("two", "default").unwrap()
}

#[tokio::test]
async fn a_same_uid_client_is_answered() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let session = hello(&worker.socket, default_instance()).await.expect("answered");
    assert_eq!(session.worker.generation.hex(), worker.generation);
    assert_eq!(session.worker.uid, uid());
    assert_eq!(session.role, StreamRole::Status);
    let socket = worker.socket.clone();
    assert!(worker.stop().success());
    assert!(!socket.exists(), "the shutdown removes its socket");
}

#[tokio::test]
async fn a_malformed_stream_closes_alone() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let mut wrong = UnixStream::connect(&worker.socket).unwrap();
    wrong.write_all(b"NOTGRAMD and more").unwrap();
    assert!(closed_within(&mut wrong, Duration::from_secs(3)), "a wrong preface");
    let mut oversized = UnixStream::connect(&worker.socket).unwrap();
    oversized.write_all(&PREFACE).unwrap();
    oversized.write_all(&(HELLO_FRAME_MAX + 1).to_be_bytes()).unwrap();
    assert!(closed_within(&mut oversized, Duration::from_secs(3)), "a hello past its frame ceiling");
    let mut truncated = UnixStream::connect(&worker.socket).unwrap();
    truncated.write_all(&PREFACE).unwrap();
    truncated.write_all(&100u32.to_be_bytes()).unwrap();
    truncated.write_all(&[0u8; 10]).unwrap();
    truncated.shutdown(std::net::Shutdown::Write).unwrap();
    assert!(closed_within(&mut truncated, Duration::from_secs(3)), "a hello cut short");
    let other = hello(&worker.socket, Instance::new("two", "other").unwrap()).await;
    assert!(other.is_err_and(|e| e.contains("WrongWorker")), "another instance's hello is refused");
    hello(&worker.socket, default_instance()).await.expect("and the next client is answered");
    assert!(worker.stop().success());
}

#[test]
fn a_silent_client_closes_at_the_hello_deadline() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let mut silent = UnixStream::connect(&worker.socket).unwrap();
    let started = Instant::now();
    assert!(closed_within(&mut silent, Duration::from_secs(10)), "closed");
    assert!(started.elapsed() >= Duration::from_secs(4), "at the deadline, not before: {:?}", started.elapsed());
    assert!(worker.stop().success());
}

#[tokio::test]
async fn a_stream_past_the_budget_closes_before_its_hello() {
    let instance = TestInstance::with_streams("two", "default", 2);
    let worker = start(&instance, "default").expect("the worker");
    let held = [UnixStream::connect(&worker.socket).unwrap(), UnixStream::connect(&worker.socket).unwrap()];
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut extra = UnixStream::connect(&worker.socket).unwrap();
    assert!(closed_within(&mut extra, Duration::from_secs(2)), "the third stream at once");
    drop(held);
    tokio::time::sleep(Duration::from_millis(300)).await;
    hello(&worker.socket, default_instance()).await.expect("a stream within the budget again");
    assert!(worker.stop().success());
}

#[test]
fn a_burst_past_the_budget_closes_at_once() {
    let instance = TestInstance::with_streams("two", "default", 2);
    let worker = start(&instance, "default").expect("the worker");
    let held = [UnixStream::connect(&worker.socket).unwrap(), UnixStream::connect(&worker.socket).unwrap()];
    std::thread::sleep(Duration::from_millis(300));
    let mut burst: Vec<UnixStream> = (0..32).map(|_| UnixStream::connect(&worker.socket).unwrap()).collect();
    for stream in &mut burst {
        assert!(closed_within(stream, Duration::from_secs(2)), "each stream past the budget");
    }
    drop(held);
    assert!(worker.stop().success());
}

#[test]
fn a_live_listener_at_the_socket_is_refused() {
    let instance = TestInstance::new("two", "default");
    let socket = instance.load("default").paths.socket.unwrap();
    std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
    std::fs::set_permissions(socket.parent().unwrap(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let live = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let refused = start(&instance, "default").err().expect("a live listener answers there");
    assert_eq!(refused.status.code(), Some(73), "{refused:?}");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("a live listener"), "{refused:?}");
    assert!(socket.exists(), "and its socket is left");
    drop(live);
}

#[tokio::test]
async fn two_data_homes_cannot_share_one_socket() {
    let instance = TestInstance::new("two", "default");
    let first = start(&instance, "default").expect("the first worker");
    let mut env = instance.env.clone();
    let other = instance.home("other-data");
    std::fs::create_dir_all(&other).unwrap();
    env.insert("UENV_USR_DATA".to_string(), other.to_string_lossy().into_owned());
    let refused = start_with(&instance, "default", &env).err().expect("the socket's lock is held");
    assert_eq!(refused.status.code(), Some(73), "{refused:?}");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("another worker holds its lock"), "{refused:?}");
    hello(&first.socket, default_instance()).await.expect("the first worker still answers");
    assert!(first.stop().success());
}

#[test]
fn the_socket_and_its_directory_are_private() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let meta = std::fs::symlink_metadata(&worker.socket).unwrap();
    assert!(meta.file_type().is_socket());
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    let dir = std::fs::metadata(worker.socket.parent().unwrap()).unwrap();
    assert_eq!(dir.permissions().mode() & 0o777, 0o700);
    assert!(worker.stop().success());
}

#[test]
fn a_start_without_a_runtime_home_is_refused() {
    let instance = TestInstance::new("two", "default");
    let mut env = instance.env.clone();
    env.remove("UENV_USR_TRANSIENT");
    let refused = start_with(&instance, "default", &env).err().expect("refused");
    assert_eq!(refused.status.code(), Some(78), "{refused:?}");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("no runtime home"), "{refused:?}");
}

#[test]
fn a_dead_workers_socket_is_replaced_and_a_foreign_file_refused() {
    let instance = TestInstance::new("two", "default");
    let socket = instance.load("default").paths.socket.unwrap();
    std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
    std::fs::set_permissions(socket.parent().unwrap(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(&socket, "").unwrap();
    let refused = start(&instance, "default").err().expect("a plain file at the socket's path");
    assert_eq!(refused.status.code(), Some(73), "{refused:?}");
    std::fs::remove_file(&socket).unwrap();
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    let worker = start(&instance, "default").expect("a dead worker's socket is replaced");
    assert!(worker.stop().success());
}
