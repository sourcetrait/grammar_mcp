//! grammard's commands: those this build lacks refuse at once, opening nothing.

use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn every_unbuilt_command_refuses_as_unimplemented() {
    for command in ["instance", "bridge", "cli"] {
        let started = Instant::now();
        let out = Command::new(env!("CARGO_BIN_EXE_grammard"))
            .arg(command)
            .output()
            .expect("grammard runs");
        assert_eq!(out.status.code(), Some(69), "{command}: {out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!("grammard: {command} is not implemented in this build")),
            "{command}: {stderr}",
        );
        assert!(out.stdout.is_empty(), "{command} writes nothing to stdout");
        assert!(started.elapsed() < Duration::from_secs(10), "{command} returns at once");
    }
}

#[test]
fn serve_takes_its_instance_by_name() {
    let out = Command::new(env!("CARGO_BIN_EXE_grammard")).arg("serve").output().expect("grammard runs");
    assert_eq!(out.status.code(), Some(2), "a usage error: {out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("--id"), "{out:?}");
}
