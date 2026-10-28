//! The host's scratch directories and their lifecycle.
//!
//! Private to its process, exported to every definition, removed at an
//! ordinary end, and swept once their owner is gone.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use nix::sys::signal::Signal;
use sourcetrait_grammar_tests::{Host, envelope, run_output_env};
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// A test's own directory, private as the platform's are.
fn private_dir(dir: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&dir).expect("a base directory");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .expect("the base's mode");
    dir
}

/// A test's own temporary and runtime directories.
fn bases(root: &Path) -> (PathBuf, PathBuf) {
    (private_dir(root.join("tmp")), private_dir(root.join("run")))
}

/// A path as the env a host is spawned with takes it.
fn text(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// A host whose UENV homes name `tmp` and `run`, the standard ones decoys.
fn spawn_on(scratch: &Path, id: &str, tmp: &Path, run: &Path) -> Host {
    let decoy_tmp = private_dir(scratch.join("decoy").join("tmp"));
    let decoy_run = private_dir(scratch.join("decoy").join("run"));
    Host::spawn_full(
        scratch,
        &["--id", id],
        &[
            ("UENV_USR_TMP", text(tmp)),
            ("UENV_USR_TRANSIENT", text(run)),
            ("TMPDIR", text(&decoy_tmp)),
            ("XDG_RUNTIME_DIR", text(&decoy_run)),
        ],
    )
}

/// The host's bulk and transient directories, as `interface info` reports them.
fn scratch_dirs(host: &mut Host) -> (PathBuf, PathBuf) {
    let info = host.info();
    let field = |name: &str| {
        PathBuf::from(
            envelope(&info)["result"][name]
                .as_str()
                .unwrap_or_else(|| panic!("info carries {name}; got {info}")),
        )
    };
    (field("tmp"), field("transient"))
}

/// The scratch directories `base` holds.
fn scratch_in(base: &Path) -> usize {
    std::fs::read_dir(base)
        .expect("the base reads")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("grammar-mcp."))
        .count()
}

/// A private directory in `base`, under the sweep's prefix, marked by its host.
fn assert_scratch(dir: &Path, base: &Path, pid: u32) {
    assert_eq!(dir.parent(), Some(base), "{} sits in {}", dir.display(), base.display());
    let meta = std::fs::metadata(dir).expect("the scratch directory exists");
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
    let pid = pid.to_string();
    assert_eq!(
        owner.split_whitespace().next(),
        Some(pid.as_str()),
        "the marker names its host's pid, then its start time; got {owner:?}",
    );
    assert_eq!(owner.split_whitespace().count(), 2, "got {owner:?}");
}

/// The UENV homes come first, so the standard variables' decoys stay empty.
#[tested]
fn both_directories_are_private_marked_and_exported() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let scratch = t.temp_dir().join("host");
    let mut host = spawn_on(&scratch, "scratchown", &tmp, &run);
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert_scratch(&tmp_dir, &tmp, host.pid());
    assert_scratch(&transient_dir, &run, host.pid());
    for decoy in ["tmp", "run"] {
        let decoy = scratch.join("decoy").join(decoy);
        assert_eq!(scratch_in(&decoy), 0, "nothing in {}", decoy.display());
    }

    let resp = host.execute(
        "def execute [data: nothing]: nothing -> record<tmp: string, transient: string> { \
         { tmp: $env.EQUIP_TMP, transient: $env.EQUIP_TRANSIENT } }",
        "null",
    );
    let result = &envelope(&resp)["result"];
    assert_eq!(
        result["tmp"].as_str(),
        tmp_dir.to_str(),
        "a definition reads the bulk one as $env.EQUIP_TMP; got {resp}",
    );
    assert_eq!(
        result["transient"].as_str(),
        transient_dir.to_str(),
        "and the transient one as $env.EQUIP_TRANSIENT; got {resp}",
    );
}

/// With no UENV homes, `TMPDIR` and `XDG_RUNTIME_DIR` name the bases.
#[tested]
fn the_standard_variables_stand_behind_the_uenv_ones() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut host = Host::spawn_full(
        &t.temp_dir().join("host"),
        &["--id", "scratchstd"],
        &[
            ("UENV_USR_TMP", ""),
            ("UENV_USR_TRANSIENT", ""),
            ("TMPDIR", text(&tmp)),
            ("XDG_RUNTIME_DIR", text(&run)),
        ],
    );
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert_scratch(&tmp_dir, &tmp, host.pid());
    assert_scratch(&transient_dir, &run, host.pid());
}

/// With no runtime directory, transient files go under the temporary one.
///
/// The host warns of it, as the XDG spec asks.
#[tested]
fn transient_files_fall_back_to_the_temporary_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, _) = bases(t.temp_dir());
    let log = t.temp_dir().join("stderr.log");
    let mut host = Host::spawn_logged(
        &t.temp_dir().join("host"),
        &["--id", "scratchfall"],
        &[
            ("UENV_USR_TMP", text(&tmp)),
            ("UENV_USR_TRANSIENT", ""),
            ("XDG_RUNTIME_DIR", ""),
        ],
        &log,
    );
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert_scratch(&tmp_dir, &tmp, host.pid());
    assert_scratch(&transient_dir, &tmp, host.pid());
    assert_ne!(tmp_dir, transient_dir, "two directories still, side by side");
    let stderr = std::fs::read_to_string(&log).expect("the host's stderr");
    assert!(stderr.contains("XDG_RUNTIME_DIR"), "the fallback is warned of; got {stderr:?}");
}

/// A temporary directory that doesn't exist fails the start.
///
/// The platform's directory is checked, never made.
#[tested]
fn a_missing_temporary_directory_fails_the_start() {
    let t = testing::test!({ .using_temp_dir() });
    let missing = t.temp_dir().join("absent");
    let out = run_output_env(
        &t.temp_dir().join("host"),
        &["--id", "scratchmiss", "cli", "interface", "interface info"],
        &[("UENV_USR_TMP", text(&missing))],
    );
    assert_eq!(out.status.code(), Some(2), "the start fails; got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(text(&missing)),
        "the refusal names {}; got {stderr}",
        missing.display(),
    );
    assert!(!missing.exists(), "{} is never made", missing.display());
}

#[tested]
fn sigterm_removes_both_directories() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut host = spawn_on(&t.temp_dir().join("host"), "scratchterm", &tmp, &run);
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert!(tmp_dir.is_dir(), "{} exists while its host runs", tmp_dir.display());
    assert!(transient_dir.is_dir(), "{} exists while its host runs", transient_dir.display());

    let status = host
        .signal_and_wait(Signal::SIGTERM, Duration::from_secs(15))
        .expect("a signalled host should exit rather than hang");
    assert_eq!(status.code(), Some(128 + Signal::SIGTERM as i32), "the handler ran");
    assert!(!tmp_dir.exists(), "the signal sweep removes {}", tmp_dir.display());
    assert!(!transient_dir.exists(), "and {}", transient_dir.display());
}

/// A dead host's directories go at the next start.
///
/// A host dropped is killed outright, which runs no exit hook, so its
/// directories outlive it until the next start sweeps them.
#[tested]
fn a_dead_hosts_directories_are_swept_at_start() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut live = spawn_on(&t.temp_dir().join("live"), "scratchlive", &tmp, &run);
    let (live_tmp, live_transient) = scratch_dirs(&mut live);
    let mut dead = spawn_on(&t.temp_dir().join("dead"), "scratchdead", &tmp, &run);
    let (dead_tmp, dead_transient) = scratch_dirs(&mut dead);
    drop(dead);
    assert!(dead_tmp.is_dir(), "a host killed outright leaves {}", dead_tmp.display());
    assert!(dead_transient.is_dir(), "and {}", dead_transient.display());

    let _next = spawn_on(&t.temp_dir().join("next"), "scratchnext", &tmp, &run);
    assert!(!dead_tmp.exists(), "the next start sweeps {}", dead_tmp.display());
    assert!(!dead_transient.exists(), "and {}", dead_transient.display());
    assert!(live_tmp.is_dir(), "and keeps the live host's {}", live_tmp.display());
    assert!(live_transient.is_dir(), "and {}", live_transient.display());
}
