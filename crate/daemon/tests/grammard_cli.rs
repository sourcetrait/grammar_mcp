//! grammard's commands: each refuses at once as unimplemented, opening nothing.

use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn every_command_refuses_as_unimplemented() {
    for command in ["serve", "instance", "bridge", "cli"] {
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
