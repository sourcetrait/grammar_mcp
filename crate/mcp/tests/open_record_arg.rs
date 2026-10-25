use serde_json::json;
use sourcetrait_grammar_mcp::guts::{TestServer, error_text, has_error, write_tree};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

#[test]
fn run_binds_open_record_arg() {
    let s = TESTING.harness();
    let env = s.run(
        json!({"extra": {}}),
        json!({"cols": "int"}),
        json!({"extra": {"a": 1, "b": "x"}}),
        "{ cols: ($args.extra | columns | length) }",
    );
    assert_eq!(env["result"]["cols"].as_i64(), Some(2), "open-record arg should bind an arbitrary record; got {env}");
}

#[test]
fn run_binds_empty_open_record_arg() {
    let s = TESTING.harness();
    let env = s.run(
        json!({"extra": {}}),
        json!({"cols": "int"}),
        json!({"extra": {}}),
        "{ cols: ($args.extra | columns | length) }",
    );
    assert_eq!(env["result"]["cols"].as_i64(), Some(0), "got {env}");
}

#[test]
fn run_rejects_non_record_open_arg() {
    let s = TESTING.harness();
    let env = s.run(
        json!({"extra": {}}),
        json!({"cols": "int"}),
        json!({"extra": 5}),
        "{ cols: 0 }",
    );
    assert!(has_error(&env), "a non-record `extra` should be rejected; got {env}");
}

#[test]
fn run_result_open_record_still_denied() {
    let s = TESTING.harness();
    let env = s.run(
        json!({"noop": "int"}),
        json!({"extra": {}}),
        json!({"noop": 0}),
        "{ extra: {} }",
    );
    assert!(has_error(&env), "result open record should deny; got {env}");
    assert!(error_text(&env).contains("nested empty record"), "expected the nested-empty-record denial; got {env}");
}

#[tested]
fn commit_inspect_and_call_open_record_arg_field() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/openlib");
    write_tree(
        &src,
        &[
            ("mod.nu", "export module m\n"),
            ("m/mod.nu", "export use tally\n"),
            (
                "m/tally/mod.nu",
                "export def main [args: record<x: int, extra: record<>>]: nothing -> record<sum: int, extracols: int> {\n    { sum: $args.x, extracols: ($args.extra | columns | length) }\n}\n",
            ),
        ],
    );
    let committed = s.commit("sourcetrait/openlib");
    assert!(!has_error(&committed), "a call-target with a `record<>` arg field should commit; got {committed}");

    let inspected = s.inspect("sourcetrait/openlib:m:tally");
    assert_eq!(
        inspected["doc"]["signature"].as_str(),
        Some("sourcetrait/openlib:m:tally <x:int,extra:record<>> <sum:int,extracols:int>"),
        "the OPEN record keeps its `record<>` spelling inside the signature - it is \
         the author's literal arg syntax, not a bare `record`; got {inspected}",
    );

    let called = s.call("sourcetrait/openlib:m:tally", json!({"x": 5, "extra": {"a": 1, "b": 2, "c": 3}}));
    assert_eq!(called["result"]["sum"].as_i64(), Some(5), "got {called}");
    assert_eq!(called["result"]["extracols"].as_i64(), Some(3), "got {called}");
}
