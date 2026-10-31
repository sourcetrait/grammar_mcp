//! grammard-probe's commands: the inventory, and a case's run that never passes
//! by omission.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use nutype::NuTypeFile;
use nuval::NuVal;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// A candidate set's manifest its model holds.
const SET: &str = "{schema_version: 1}";

/// A fixture's descriptor its model holds: a run with no endpoints yet.
const FIXTURE: &str = "{schema_version: 1, run: r1, lab_admin: {via: ssh, scope: fixture}, \
     candidates: null, endpoints: [], mtls: null}";

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

/// An output read back and held to its model, `type/<model>`.
fn read_output(path: &Path, model: &str) -> NuVal {
    let value = NuVal::from_nuon(&fs::read_to_string(path).expect("an output")).expect("NUON");
    let model_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("type").join(model);
    let model_text = fs::read_to_string(&model_path).expect("its model");
    let model_file = NuTypeFile::from_text(&model_text).expect("a model");
    model_file.ty.check(&value).unwrap_or_else(|e| panic!("{model}: {e}"));
    value
}

/// A record's field.
fn field<'a>(value: &'a NuVal, name: &str) -> &'a NuVal {
    let NuVal::Record(fields) = value else { panic!("not a record: {value:?}") };
    fields.iter().find(|(k, _)| k == name).map(|(_, v)| v).unwrap_or_else(|| panic!("no {name}"))
}

#[tested]
fn the_inventory_lists_every_group_wire_alone_implemented() {
    let t = testing::test!({ .using_temp_dir() });
    let out = t.temp_dir().join("inventory.nuon");
    let run = probe(&["inventory", "--out", out.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let inventory = read_output(&out, "Inventory.nutype");
    let NuVal::List(groups) = field(&inventory, "groups") else { panic!("groups") };
    assert_eq!(groups.len(), 14);
    for group in groups {
        let wire = *field(group, "case") == NuVal::String("wire".to_string());
        assert_eq!(*field(group, "implemented"), NuVal::Bool(wire), "{group:?}");
        let NuVal::List(assertions) = field(group, "assertions") else { panic!("assertions") };
        assert_eq!(assertions.is_empty(), !wire, "{group:?}");
    }
    let NuVal::Record(capabilities) = field(&inventory, "capabilities") else { panic!("capabilities") };
    assert!(!capabilities.is_empty() && capabilities.iter().all(|(_, on)| *on == NuVal::Bool(false)));

    let again = probe(&["inventory", "--out", out.to_str().unwrap()]);
    assert_eq!(again.status.code(), Some(78), "an existing output is never overwritten");
}

#[tested]
fn an_unimplemented_case_ends_unrun() {
    let t = testing::test!({ .using_temp_dir() });
    let dir = t.temp_dir();
    let set = write(dir, "set.nuon", SET);
    let fixture = write(dir, "fixture.nuon", FIXTURE);
    let out = dir.join("runtime");
    let run = probe(&[
        "run", "--case", "runtime", "--candidate-set", &set, "--fixture", &fixture,
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(run.status.code(), Some(77), "{run:?}");
    let result = read_output(&out.join("result.nuon"), "Result.nutype");
    assert_eq!(*field(&result, "status"), NuVal::String("unrun".to_string()));
    assert_eq!(*field(&result, "exit"), NuVal::Int(77));
    assert_eq!(*field(&result, "case"), NuVal::String("runtime".to_string()));
}

#[tested]
fn the_wire_case_passes_every_assertion() {
    let t = testing::test!({ .using_temp_dir() });
    let dir = t.temp_dir();
    let set = write(dir, "set.nuon", SET);
    let fixture = write(dir, "fixture.nuon", FIXTURE);
    let out = dir.join("wire");
    let run = probe(&[
        "run", "--case", "wire", "--candidate-set", &set, "--fixture", &fixture,
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let result = read_output(&out.join("result.nuon"), "Result.nutype");
    assert_eq!(*field(&result, "status"), NuVal::String("pass".to_string()));
    assert_eq!(*field(&result, "exit"), NuVal::Int(0));
    let NuVal::List(assertions) = field(&result, "assertions") else { panic!("assertions") };
    let names: Vec<_> = assertions.iter().map(|a| match a { NuVal::String(s) => s.as_str(), _ => panic!("a name") }).collect();
    assert_eq!(names, ["wire.manifest", "wire.vectors", "wire.types", "wire.limits", "wire.handshake", "wire.execution"]);
}

#[tested]
fn an_unknown_case_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let dir = t.temp_dir();
    let set = write(dir, "set.nuon", SET);
    let fixture = write(dir, "fixture.nuon", FIXTURE);
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
    let set = write(dir, "set.nuon", SET);
    let v2 = FIXTURE.replace("schema_version: 1", "schema_version: 2");
    let cases = [
        ("bad.nuon", "{schema_version"),
        ("v2.nuon", v2.as_str()),
        ("thin.nuon", SET),
        ("extra.nuon", "{schema_version: 1, run: r1, lab_admin: {via: ssh, scope: fixture}, \
             candidates: null, endpoints: [], mtls: null, extra: true}"),
    ];
    for (name, text) in cases {
        let fixture = write(dir, name, text);
        let out = dir.join(name.replace(".nuon", ""));
        let run = probe(&[
            "run", "--case", "client", "--candidate-set", &set, "--fixture", &fixture,
            "--out", out.to_str().unwrap(),
        ]);
        assert_eq!(run.status.code(), Some(78), "{name}: {run:?}");
        assert!(!out.exists(), "{name}: a refused run writes no result");
    }
}
