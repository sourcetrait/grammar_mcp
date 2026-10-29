//! grammard-probe's commands: the inventory, and a case's run that never passes
//! by omission.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

fn probe(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grammard-probe"))
        .args(args)
        .output()
        .expect("grammard-probe runs")
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).expect("a manifest written");
    path.to_string_lossy().into_owned()
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).expect("an output")).expect("JSON")
}

#[tested]
fn the_inventory_lists_every_group_unimplemented() {
    let t = testing::test!({ .using_temp_dir() });
    let out = t.temp_dir().join("inventory.json");
    let run = probe(&["inventory", "--out", out.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let inventory = read_json(&out);
    let groups = inventory["groups"].as_array().expect("groups");
    assert_eq!(groups.len(), 14);
    assert!(groups.iter().all(|g| g["implemented"] == false), "{groups:?}");
    let capabilities = inventory["capabilities"].as_object().expect("capabilities");
    assert!(!capabilities.is_empty() && capabilities.values().all(|on| on == false));

    let again = probe(&["inventory", "--out", out.to_str().unwrap()]);
    assert_eq!(again.status.code(), Some(78), "an existing output is never overwritten");
}

#[tested]
fn an_unimplemented_case_ends_unrun() {
    let t = testing::test!({ .using_temp_dir() });
    let dir = t.temp_dir();
    let set = write(dir, "set.json", r#"{"schema_version": 1}"#);
    let fixture = write(dir, "fixture.json", r#"{"schema_version": 1}"#);
    let out = dir.join("wire");
    let run = probe(&[
        "run", "--case", "wire", "--candidate-set", &set, "--fixture", &fixture,
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(run.status.code(), Some(77), "{run:?}");
    let result = read_json(&out.join("result.json"));
    assert_eq!(result["status"], "unrun");
    assert_eq!(result["exit"], 77);
    assert_eq!(result["case"], "wire");
}

#[tested]
fn an_unknown_case_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let dir = t.temp_dir();
    let set = write(dir, "set.json", r#"{"schema_version": 1}"#);
    let fixture = write(dir, "fixture.json", r#"{"schema_version": 1}"#);
    let out = dir.join("smoke");
    let run = probe(&[
        "run", "--case", "smoke", "--candidate-set", &set, "--fixture", &fixture,
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(run.status.code(), Some(78), "{run:?}");
    assert!(!out.exists(), "a refused run writes no result");
}

#[tested]
fn a_malformed_manifest_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let dir = t.temp_dir();
    let set = write(dir, "set.json", r#"{"schema_version": 1}"#);
    for (name, text) in [("bad.json", "not json"), ("v2.json", r#"{"schema_version": 2}"#)] {
        let fixture = write(dir, name, text);
        let out = dir.join(name.replace(".json", ""));
        let run = probe(&[
            "run", "--case", "client", "--candidate-set", &set, "--fixture", &fixture,
            "--out", out.to_str().unwrap(),
        ]);
        assert_eq!(run.status.code(), Some(78), "{name}: {run:?}");
        assert!(!out.exists(), "{name}: a refused run writes no result");
    }
}
