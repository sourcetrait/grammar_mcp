//! The controller's lease on the wire: one claim, a takeover, renewal, and binding.

mod support;

use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use sourcetrait_grammar_protocol::{
    ClientInstanceId, Expectation, FrameCodec, HELLO_BUDGET, HelloReply, Instance, LIMITS_DEFAULT,
    LeaseBinding, LeaseGrant, LeaseIntent, LeaseReply, LeaseRequest, Ping, RpcAnswer, RpcFrame, StreamRole,
    TransportError, WorkerGeneration, check_reply, decode_payload, framed, payload,
};
use support::{TestInstance, start};

type Framed = tokio_util::codec::Framed<tokio::net::UnixStream, FrameCodec>;

fn uid() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// A stream of `role` opened with `lease`, and its binding or its refusal.
async fn open(
    socket: &Path,
    role: StreamRole,
    lease: LeaseIntent,
) -> (Framed, Result<(LeaseBinding, WorkerGeneration), String>) {
    let stream = tokio::net::UnixStream::connect(socket).await.expect("a connect");
    let mut framed = framed(stream);
    let expect = Expectation {
        uid: uid(),
        instance: Instance::new("two", "default").unwrap(),
        role,
        lease,
        requested: LIMITS_DEFAULT,
        required: Vec::new(),
        last: None,
    };
    framed.send(payload(&expect.hello(ClientInstanceId([5; 16])))).await.expect("the hello");
    let frame = framed.next().await.expect("a reply").expect("its frame");
    let reply: HelloReply = decode_payload(&frame, HELLO_BUDGET).expect("a reply decoded");
    let bound = check_reply(reply, &expect).map(|session| (session.lease, session.worker.generation));
    (framed, bound.map_err(|e| format!("{e:?}")))
}

/// A claim or a takeover's stream and its grant.
async fn granted(socket: &Path, intent: LeaseIntent) -> (Framed, LeaseGrant, WorkerGeneration) {
    let (framed, bound) = open(socket, StreamRole::Rpc, intent).await;
    let (LeaseBinding::Granted(grant), generation) = bound.expect("granted") else { panic!("a grant") };
    (framed, grant, generation)
}

/// A Status stream's hello answered, or none where the worker closed it first.
async fn observed(socket: &Path) -> bool {
    let Ok(stream) = tokio::net::UnixStream::connect(socket).await else { return false };
    let mut framed = framed(stream);
    let expect = Expectation {
        uid: uid(),
        instance: Instance::new("two", "default").unwrap(),
        role: StreamRole::Status,
        lease: LeaseIntent::Observe,
        requested: LIMITS_DEFAULT,
        required: Vec::new(),
        last: None,
    };
    if framed.send(payload(&expect.hello(ClientInstanceId([6; 16])))).await.is_err() {
        return false;
    }
    let Ok(Some(Ok(frame))) = tokio::time::timeout(Duration::from_secs(5), framed.next()).await else { return false };
    decode_payload::<HelloReply>(&frame, HELLO_BUDGET).is_ok_and(|reply| check_reply(reply, &expect).is_ok())
}

/// Pings written to `stream` and none of its answers read, until the worker's sends wait on it.
///
/// A write that would block is tried again after a pause: the worker may still
/// be reading, and only a stream it has stopped reading stays full.
fn flood(stream: tokio::net::UnixStream) -> std::os::unix::net::UnixStream {
    let mut stream = stream.into_std().expect("a std stream");
    stream.set_nonblocking(true).unwrap();
    let body = payload(&RpcFrame::Ping(Ping { nonce: 1 }));
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(&body);
    let started = Instant::now();
    let mut stalls = 0;
    while stalls < 3 {
        assert!(started.elapsed() < Duration::from_secs(30), "the worker never stopped reading");
        match stream.write(&frame) {
            Ok(n) if n == frame.len() => stalls = 0,
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                stalls += 1;
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => panic!("a ping's write: {e}"),
        }
    }
    stream
}

/// The next answer on an RPC stream, within a few seconds.
async fn answer(framed: &mut Framed) -> RpcAnswer {
    let frame = tokio::time::timeout(Duration::from_secs(5), framed.next())
        .await
        .expect("an answer in time")
        .expect("an answer")
        .expect("its frame");
    decode_payload(&frame, HELLO_BUDGET).expect("an answer decoded")
}

#[tokio::test]
async fn one_claim_is_granted_and_a_second_refused_busy() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let (_held, _grant, _) = granted(&worker.socket, LeaseIntent::Claim).await;
    let (_, second) = open(&worker.socket, StreamRole::Rpc, LeaseIntent::Claim).await;
    assert!(second.is_err_and(|e| e.contains("Busy")), "a claim beside the live lease");
    assert!(worker.stop().success());
}

#[tokio::test]
async fn a_takeover_closes_the_old_controllers_stream() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let (mut old, old_grant, _) = granted(&worker.socket, LeaseIntent::Claim).await;
    let (_new, new_grant, _) = granted(&worker.socket, LeaseIntent::Takeover).await;
    assert_ne!(new_grant.lease, old_grant.lease);
    assert!(new_grant.epoch.0 > old_grant.epoch.0);
    let RpcAnswer::Close(close) = answer(&mut old).await else { panic!("the old stream closed") };
    assert!(matches!(close.error, Some(TransportError::LeaseInvalid { reason }) if reason.contains("taken over")));
    assert!(worker.stop().success());
}

#[tokio::test]
async fn a_controller_that_never_reads_is_closed_at_a_takeover() {
    let instance = TestInstance::with_streams("two", "default", 2);
    let worker = start(&instance, "default").expect("the worker");
    let (old, _, _) = granted(&worker.socket, LeaseIntent::Claim).await;
    let flooded = flood(old.into_inner());
    let (_new, _, _) = granted(&worker.socket, LeaseIntent::Takeover).await;
    let started = Instant::now();
    let mut freed = false;
    while !freed && started.elapsed() < Duration::from_secs(10) {
        freed = observed(&worker.socket).await;
        if !freed {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    assert!(freed, "the old stream's place is freed with nothing of it read");
    drop(flooded);
    assert!(worker.stop().success());
}

#[tokio::test]
async fn a_heartbeat_renews_and_a_release_frees_the_claim() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let (mut rpc, grant, _) = granted(&worker.socket, LeaseIntent::Claim).await;
    rpc.send(payload(&RpcFrame::Lease(LeaseRequest::Heartbeat { lease: grant.lease, token: grant.token })))
        .await
        .unwrap();
    assert!(matches!(answer(&mut rpc).await, RpcAnswer::Lease(Ok(LeaseReply::Renewed { .. }))));
    rpc.send(payload(&RpcFrame::Ping(Ping { nonce: 9 }))).await.unwrap();
    assert!(matches!(answer(&mut rpc).await, RpcAnswer::Pong(Ping { nonce: 9 })));
    rpc.send(payload(&RpcFrame::Lease(LeaseRequest::Release { lease: grant.lease, token: grant.token })))
        .await
        .unwrap();
    assert!(matches!(answer(&mut rpc).await, RpcAnswer::Lease(Ok(LeaseReply::Released))));
    assert!(matches!(answer(&mut rpc).await, RpcAnswer::Close(_)), "the released lease's stream closes");
    let (_again, again, _) = granted(&worker.socket, LeaseIntent::Claim).await;
    assert_ne!(again.lease, grant.lease, "a new claim after the release");
    assert!(worker.stop().success());
}

#[tokio::test]
async fn a_resume_and_an_attach_bind_to_the_live_lease() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let (first, grant, generation) = granted(&worker.socket, LeaseIntent::Claim).await;
    drop(first);
    let resume = LeaseIntent::Resume { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation };
    let (_rpc, resumed) = open(&worker.socket, StreamRole::Rpc, resume).await;
    let (LeaseBinding::Granted(resumed), _) = resumed.expect("resumed") else { panic!("a grant") };
    assert_eq!(resumed.lease, grant.lease, "the same lease, on a new stream");
    let attach = LeaseIntent::Attach { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation };
    let (_events, attached) = open(&worker.socket, StreamRole::Events, attach).await;
    assert!(
        matches!(attached, Ok((LeaseBinding::Attached { lease, .. }, _)) if lease == grant.lease),
        "an Events stream attached",
    );
    assert!(worker.stop().success());
}

#[tokio::test]
async fn a_lease_named_wrongly_is_refused() {
    let instance = TestInstance::new("two", "default");
    let worker = start(&instance, "default").expect("the worker");
    let (_held, grant, generation) = granted(&worker.socket, LeaseIntent::Claim).await;
    let wrong = [
        LeaseIntent::Resume { lease: grant.lease, token: sourcetrait_grammar_protocol::LeaseToken([0; 32]), epoch: grant.epoch, generation },
        LeaseIntent::Resume { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation: WorkerGeneration([0; 16]) },
        LeaseIntent::Attach {
            lease: grant.lease,
            token: grant.token,
            epoch: sourcetrait_grammar_protocol::LeaseEpoch(grant.epoch.0 + 1),
            generation,
        },
    ];
    for intent in wrong {
        let role = if matches!(intent, LeaseIntent::Attach { .. }) { StreamRole::Events } else { StreamRole::Rpc };
        let (_, bound) = open(&worker.socket, role, intent.clone()).await;
        assert!(bound.is_err(), "{intent:?} refused");
    }
    assert!(worker.stop().success());
}
