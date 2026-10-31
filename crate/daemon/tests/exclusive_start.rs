//! A namespace's one worker: the lock held for life, a second start refused.

mod support;

use std::io::{BufRead, BufReader};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::process::{Child, Command, Output, Stdio};

use support::TestInstance;

/// A worker serving, its pid and the generation its ready line names.
struct Worker {
    child: Child,
    generation: String,
}

impl Worker {
    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Stopped as a service stop does, its exit answered.
    fn stop(mut self) -> std::process::ExitStatus {
        let pid = nix::unistd::Pid::from_raw(self.child.id() as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM).expect("the stop");
        self.child.wait().expect("its exit")
    }

    /// Killed outright, which runs no cleanup.
    fn kill(mut self) {
        self.child.kill().expect("the kill");
        self.child.wait().expect("reaped");
    }
}

/// `namespace`'s worker started, ready, or the output of the start that failed.
fn start(instance: &TestInstance, namespace: &str) -> Result<Worker, Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_grammard"))
        .args(["serve", "--id", &instance.id, "--namespace", namespace])
        .env_clear()
        .envs(&instance.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("grammard runs");
    let mut stderr = child.stderr.take().unwrap();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
        text
    });
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    while let Some(Ok(line)) = lines.next() {
        if let Some((_, rest)) = line.split_once("at generation ") {
            let generation = rest.split(',').next().unwrap_or_default().to_string();
            std::thread::spawn(move || lines.for_each(drop));
            return Ok(Worker { child, generation });
        }
    }
    let status = child.wait().expect("the failed start's exit");
    let stderr = errors.join().unwrap_or_default().into_bytes();
    Err(Output { status, stdout: Vec::new(), stderr })
}

#[test]
fn one_identity_starts_one_worker() {
    let instance = TestInstance::new("two", "default");
    let first = start(&instance, "default").expect("the first worker");
    let refused = match start(&instance, "default") {
        Err(out) => out,
        Ok(second) => panic!("a second worker served pid {}", second.pid()),
    };
    assert_eq!(refused.status.code(), Some(75), "{refused:?}");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("the namespace is held"), "{stderr}");
    assert!(stderr.contains(&format!("pid: {}", first.pid())), "the holder named: {stderr}");
    assert_eq!(TestInstance::worker_dirs(&instance.home("tmp")).len(), 1, "the refused start made no scratch");
    let lock = instance.load("default").paths.lock;
    let status = first.stop();
    assert!(status.success(), "{status:?}");
    assert!(TestInstance::worker_dirs(&instance.home("tmp")).is_empty(), "the stop removes its scratch");
    assert!(lock.is_file(), "and leaves its lock's file");
}

#[test]
fn two_namespaces_start_apart() {
    let instance = TestInstance::new("two", "default");
    instance.add("other");
    let default = start(&instance, "default").expect("default's worker");
    let other = start(&instance, "other").expect("other's worker, its lock its own");
    assert_ne!(instance.load("default").paths.lock, instance.load("other").paths.lock);
    assert_ne!(default.generation, other.generation, "a generation each");
    assert!(default.stop().success());
    assert!(other.stop().success());
}

#[test]
fn a_killed_workers_lock_passes_to_its_restart() {
    let instance = TestInstance::new("two", "default");
    let lock = instance.load("default").paths.lock;
    let killed = start(&instance, "default").expect("the first worker");
    let inode = std::fs::metadata(&lock).unwrap().ino();
    let killed_generation = killed.generation.clone();
    killed.kill();
    let restarted = start(&instance, "default").expect("the restart takes the released lock");
    assert_eq!(std::fs::metadata(&lock).unwrap().ino(), inode, "the lock's inode kept");
    let record = std::fs::read_to_string(&lock).unwrap();
    assert!(record.contains(&format!("pid: {}", restarted.pid())), "{record}");
    assert!(record.contains(&restarted.generation), "{record}");
    assert_ne!(restarted.generation, killed_generation, "a fresh generation");
    assert_eq!(TestInstance::worker_dirs(&instance.home("tmp")).len(), 1, "the killed worker's scratch swept");
    assert!(restarted.stop().success());
}

#[test]
fn a_failed_start_releases_its_lock() {
    let instance = TestInstance::new("two", "default");
    let tmp = instance.home("tmp");
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o500)).unwrap();
    let failed = start(&instance, "default").err().expect("no scratch can be made");
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(failed.status.code(), Some(73), "{failed:?}");
    let worker = start(&instance, "default").expect("the lock was released");
    assert!(worker.stop().success());
}
