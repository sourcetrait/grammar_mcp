//! A namespace's one worker: the lock held for life, a second start refused.

mod support;

use std::os::unix::fs::{MetadataExt, PermissionsExt};

use support::{TestInstance, start};

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
    assert!(record.contains("holder: grammard"), "{record}");
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
