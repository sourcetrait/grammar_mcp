//! The host's scratch directory: private to its process, exported to every
//! definition, removed at an ordinary end, and swept once its owner is gone.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use nix::sys::signal::Signal;
use sourcetrait_grammar_tests::{Host, envelope};
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The host's scratch directory, as `interface info` reports it.
fn shm_dir(host: &mut Host) -> PathBuf {
    let info = host.info();
    PathBuf::from(
        envelope(&info)["result"]["shm_dir"]
            .as_str()
            .unwrap_or_else(|| panic!("info carries shm_dir; got {info}")),
    )
}

#[tested]
fn the_scratch_directory_is_private_owned_and_exported() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "shmown"]);
    let dir = shm_dir(&mut host);

    let meta = std::fs::metadata(&dir).expect("the scratch directory exists");
    assert!(meta.is_dir(), "{} is a directory", dir.display());
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o700,
        "only its owner may enter {}",
        dir.display(),
    );
    let name = dir.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    assert!(name.starts_with("grammar-mcp."), "the sweep matches its prefix; got {name}");

    let owner = std::fs::read_to_string(dir.join("owner")).expect("its owner marker");
    let pid = host.pid().to_string();
    assert_eq!(
        owner.split_whitespace().next(),
        Some(pid.as_str()),
        "the marker names its host's pid, then its start time; got {owner:?}",
    );
    assert_eq!(owner.split_whitespace().count(), 2, "got {owner:?}");

    let resp = host.execute(
        "def execute [data: nothing]: nothing -> record<shm: string> { { shm: $env.EQUIP_SHM_DIR } }",
        "null",
    );
    assert_eq!(
        envelope(&resp)["result"]["shm"].as_str(),
        dir.to_str(),
        "a definition reads it as $env.EQUIP_SHM_DIR; got {resp}",
    );
}

#[tested]
fn sigterm_removes_the_scratch_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "shmterm"]);
    let dir = shm_dir(&mut host);
    assert!(dir.is_dir(), "{} exists while its host runs", dir.display());

    let status = host
        .signal_and_wait(Signal::SIGTERM, Duration::from_secs(15))
        .expect("a signalled host should exit rather than hang");
    assert_eq!(status.code(), Some(128 + Signal::SIGTERM as i32), "the handler ran");
    assert!(!dir.exists(), "the signal sweep removes {}", dir.display());
}

#[tested]
fn a_dead_hosts_scratch_directory_is_swept_at_start() {
    let t = testing::test!({ .using_temp_dir() });
    let mut live = Host::spawn_args(&t.temp_dir().join("live"), &["--id", "shmlive"]);
    let live_dir = shm_dir(&mut live);
    let mut dead = Host::spawn_args(&t.temp_dir().join("dead"), &["--id", "shmdead"]);
    let dead_dir = shm_dir(&mut dead);
    // Drop kills outright, which runs no exit hook, so the directory outlives it.
    drop(dead);
    assert!(dead_dir.is_dir(), "a host killed outright leaves {}", dead_dir.display());

    let _next = Host::spawn_args(&t.temp_dir().join("next"), &["--id", "shmnext"]);
    assert!(!dead_dir.exists(), "the next start sweeps {}", dead_dir.display());
    assert!(live_dir.is_dir(), "and keeps the live host's {}", live_dir.display());
}
