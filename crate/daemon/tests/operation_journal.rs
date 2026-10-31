//! The worker's operation journal: opened at its start, held for its life, a
//! segment of its own each start.

mod support;

use std::path::PathBuf;

use support::{TestInstance, start, start_with};

/// The journal's segments under `instance`'s durable state.
fn segments(instance: &TestInstance) -> PathBuf {
    instance.load("default").paths.state.join("operations").join("segments")
}

#[test]
fn a_workers_start_opens_its_journal_and_each_start_writes_its_own_segment() {
    let instance = TestInstance::new("fixture", "default");
    let worker = start(&instance, "default").expect("the worker");
    let first = segments(&instance).join("1.nuonl");
    let header = std::fs::read_to_string(&first).expect("the first segment");
    assert_eq!(header.lines().count(), 1, "the header alone: {header}");
    assert!(header.contains(&worker.generation), "the header names the worker's generation: {header}");
    assert!(worker.stop().success());
    let restarted = start(&instance, "default").expect("the restart");
    let second = std::fs::read_to_string(segments(&instance).join("2.nuonl")).expect("a segment of its own");
    assert!(second.contains(&restarted.generation), "{second}");
    assert_eq!(std::fs::read_to_string(&first).unwrap(), header, "the first left as it was");
    assert!(restarted.stop().success());
}

#[test]
fn a_second_worker_sharing_its_state_home_is_refused() {
    let instance = TestInstance::new("fixture", "default");
    let first = start(&instance, "default").expect("the first worker");
    let mut env = instance.env.clone();
    let data = instance.home("other-data");
    std::fs::create_dir_all(&data).unwrap();
    env.insert("UENV_USR_DATA".to_string(), data.to_string_lossy().into_owned());
    let run = tempfile::Builder::new().prefix("g").tempdir_in("/tmp").expect("a runtime home");
    env.insert("UENV_USR_TRANSIENT".to_string(), run.path().to_string_lossy().into_owned());
    let refused = start_with(&instance, "default", &env).err().expect("the journal's lock is held");
    assert_eq!(refused.status.code(), Some(75), "{refused:?}");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("the operation journal is held"), "{stderr}");
    assert!(first.stop().success());
    assert_eq!(std::fs::read_dir(segments(&instance)).unwrap().count(), 1, "the refused start wrote no segment");
}
