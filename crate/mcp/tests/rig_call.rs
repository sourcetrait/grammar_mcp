use serde_json::json;
use sourcetrait_grammar_mcp::guts::{
    TestServer, has_error, rig_block, valid_function_source, write_source,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

#[tested]
fn call_after_commit_returns_result() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/calc");
    let _ = s.scaffold(&["sourcetrait/calc:math:double"]);
    write_source(
        &src,
        "math/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($args.x * 2) }"),
    );
    let _ = s.commit("sourcetrait/calc");
    let env = s.call("sourcetrait/calc:math:double", json!({"x": 7}));
    assert_eq!(env["result"]["out"].as_i64(), Some(14), "got {env}");
    let fields: Vec<&String> = env.as_object().expect("an envelope record").keys().collect();
    assert_eq!(fields, vec!["nonce", "result"], "a call's envelope is its nonce and result; got {env}");
}

#[tested]
fn a_call_result_is_lenient() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/lenientlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use rows\nexport use scalar\nexport use quiet\n");
    // `from nuon` answers `any`, so each declared type holds at parse and only the
    // value at runtime says what came back.
    write_source(
        &src,
        "m/rows/mod.nu",
        "export def main [args: nothing]: nothing -> record<i: int> { '[[i]; [1] [2]]' | from nuon }\n",
    );
    write_source(
        &src,
        "m/scalar/mod.nu",
        "export def main [args: nothing]: nothing -> record<i: int> { '5' | from nuon }\n",
    );
    write_source(
        &src,
        "m/quiet/mod.nu",
        "export def main [args: nothing]: nothing -> nothing { }\n",
    );
    let committed = s.commit("sourcetrait/lenientlib");
    assert!(!has_error(&committed), "got {committed}");

    let rows = s.call("sourcetrait/lenientlib:m:rows", json!({}));
    assert_eq!(rows["result"], json!([{"i": 1}, {"i": 2}]), "a table rides as it is; got {rows}");
    let scalar = s.call("sourcetrait/lenientlib:m:scalar", json!({}));
    assert_eq!(scalar["result"], json!({}), "anything else answers an empty record; got {scalar}");
    let quiet = s.call("sourcetrait/lenientlib:m:quiet", json!({}));
    assert!(
        quiet.get("result").is_some_and(|result| result.is_null()),
        "nothing rides as null; got {quiet}",
    );
}

#[tested]
fn call_after_module_commit_returns_result() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/importable");
    let _ = s.scaffold(&["sourcetrait/importable:util:triple"]);
    write_source(
        &src,
        "util/triple/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($args.x * 3) }"),
    );
    let _ = s.commit("sourcetrait/importable");
    let env = s.call("sourcetrait/importable:util:triple", json!({"x": 11}));
    assert_eq!(env["result"]["out"].as_i64(), Some(33), "got {env}");
}

#[test]
fn call_unknown_rig_errors() {
    let s = TESTING.harness();
    let env = s.call("sourcetrait/ghost:m:noop", json!({"noop": 0}));
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn call_missing_function_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/partlib");
    let env = s.call("sourcetrait/partlib:m:ghost", json!({"noop": 0}));
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn call_bad_module_path_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/safelib");
    for bad in [
        "sourcetrait/safelib:../etc:x",
        "sourcetrait/safelib:a/../b:x",
        "sourcetrait/safelib:/abs:x",
    ] {
        let env = s.call(bad, json!({"n": 0}));
        assert!(has_error(&env), "namepath {bad:?} should error; got {env}");
    }
}

#[tested]
fn call_args_typecheck_failure_surfaces() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/strictlib");
    let _ = s.scaffold(&["sourcetrait/strictlib:m:needs_int"]);
    write_source(
        &src,
        "m/needs_int/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $args.x }"),
    );
    let _ = s.commit("sourcetrait/strictlib");
    let env = s.call("sourcetrait/strictlib:m:needs_int", json!({"x": "five"}));
    assert!(has_error(&env), "type mismatch should surface as error; got {env}");
}

#[tested]
fn inspect_returns_function_doc() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/inspectlib");
    let _ = s.scaffold(&["sourcetrait/inspectlib:math:double"]);
    write_source(
        &src,
        "math/double/mod.nu",
        "# doubles its input\n#\n# returns the doubled value\nexport def main [args: record<x: int>]: nothing -> record<out: int> { { out: ($args.x * 2) } }\n",
    );
    let _ = s.commit("sourcetrait/inspectlib");
    let env = s.inspect("sourcetrait/inspectlib:math:double");
    assert_eq!(
        env["result"].as_object().expect("a result record").len(),
        1,
        "the result is EXACTLY one field; got {env}",
    );
    let doc = &env["result"]["doc"];
    assert_eq!(
        doc["signature"].as_str(),
        Some("sourcetrait/inspectlib:math:double <x:int> <out:int> # doubles its input"),
        "the STANDALONE form carries the full namepath, both signature groups, \
         and the summary; got {env}",
    );
    assert_eq!(doc["details"].as_str(), Some("returns the doubled value"));
    assert!(
        doc["src"]
            .as_str()
            .is_some_and(|p| p.ends_with("/equip/sourcetrait/inspectlib/rig/math/double/mod.nu")),
        "src is the COMMITTED canonical mod.nu holding main; got {env}",
    );
    assert!(
        doc.get("summary").is_none(),
        "a call has no separate summary - it rides the signature; got {env}",
    );
    assert!(
        doc.get("args_schema").is_none() && doc.get("result_schema").is_none(),
        "the structured schemas are replaced by the signature; got {env}",
    );
}

#[tested]
fn inspect_rig_root_and_module() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/inspectlib2");
    write_source(&src, "mod.nu", "# the inspectlib2 rig\nexport module math\n");
    write_source(&src, "math/mod.nu", "# math helpers\nexport use double\n");
    write_source(
        &src,
        "math/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($args.x * 2) }"),
    );
    let _ = s.commit("sourcetrait/inspectlib2");
    let lib = s.inspect("sourcetrait/inspectlib2");
    assert_eq!(lib["result"]["doc"]["summary"].as_str(), Some("the inspectlib2 rig"));
    assert!(
        lib["result"]["doc"]["srcdir"]
            .as_str()
            .is_some_and(|p| p.ends_with("/equip/sourcetrait/inspectlib2/rig")),
        "a rig carries srcdir - the committed directory it lives in; got {lib}",
    );
    let m = s.inspect("sourcetrait/inspectlib2:math");
    assert_eq!(m["result"]["doc"]["summary"].as_str(), Some("math helpers"));
    assert!(
        m["result"]["doc"]["src"]
            .as_str()
            .is_some_and(|p| p.ends_with("/equip/sourcetrait/inspectlib2/rig/math/mod.nu")),
        "a module carries src - its committed mod.nu; got {m}",
    );
}

#[tested]
fn inspect_undocumented_is_empty() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/inspectlib3");
    let _ = s.scaffold(&["sourcetrait/inspectlib3:m:f"]);
    write_source(
        &src,
        "m/f/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $args.x }"),
    );
    let _ = s.commit("sourcetrait/inspectlib3");
    let env = s.inspect("sourcetrait/inspectlib3:m:f");
    assert_eq!(
        env["result"]["doc"]["signature"].as_str(),
        Some("sourcetrait/inspectlib3:m:f <x:int> <out:int>"),
        "an undocumented call's signature carries no trailing ` # ` at all; got {env}",
    );
    assert_eq!(env["result"]["doc"]["details"].as_str(), Some(""));
}

#[test]
fn inspect_unknown_rig_errors() {
    let s = TESTING.harness();
    let env = s.inspect("sourcetrait/ghost");
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn result_record_field_shapes_preserved() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/fidelitylib");
    let _ = s.scaffold(&["sourcetrait/fidelitylib:m:shapes"]);
    write_source(
        &src,
        "m/shapes/mod.nu",
        "export def main [args: record<n: int>]: nothing -> record<p: path, d: directory, c: cell-path, g: glob> { { p: \"x\", d: \"y\", c: $.a, g: (\"z\" | into glob) } }\n",
    );
    let committed = s.commit("sourcetrait/fidelitylib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");
    let env = s.inspect("sourcetrait/fidelitylib:m:shapes");
    assert_eq!(
        env["result"]["doc"]["signature"].as_str(),
        Some(
            "sourcetrait/fidelitylib:m:shapes <n:int> \
             <p:path,d:directory,c:cell-path,g:glob>"
        ),
        "path/directory/cell-path/glob survive verbatim into the signature; got {env}",
    );
}

#[tested]
fn helper_file_pruned_from_info_and_not_callable() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/helperlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use ./util.nu\nexport use real\n");
    write_source(&src, "m/util.nu", "export def helper [n: int] { $n * 2 }\n");
    write_source(
        &src,
        "m/real/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($args.x + 1) }"),
    );
    let committed = s.commit("sourcetrait/helperlib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");

    let info = s.info();
    let signatures = info["result"]["signatures"].as_str().expect("signatures block");
    assert_eq!(
        rig_block(signatures, "helperlib"),
        " helperlib\n  m\n   real <x:int> <out:int>\n",
        "only the call-target is listed - the organizational helper file is \
         pruned from the index, so it never reaches the block",
    );

    let ok = s.call("sourcetrait/helperlib:m:real", json!({"x": 41}));
    assert_eq!(ok["result"]["out"].as_i64(), Some(42), "got {ok}");

    let bad = s.call("sourcetrait/helperlib:m:util", json!({"n": 5}));
    assert!(
        has_error(&bad),
        "an organizational helper file must NOT be callable; got {bad}",
    );
}
