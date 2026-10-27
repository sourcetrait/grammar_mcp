//! The `nu` tool end to end through the in-process harness: its lanes, its
//! checks, its timeout, and `rerun` over its definitions.

use serde_json::{Value, json};
use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{TestServer, error_kind, error_messages, has_error};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

fn nonce_of(env: &Value) -> String {
    env["nonce"]
        .as_str()
        .or_else(|| env["error"]["nonce"].as_str())
        .unwrap_or_else(|| panic!("no nonce in {env}"))
        .to_string()
}

#[test]
fn a_record_round_trips_on_the_stateless_pool() {
    let s = TESTING.harness();
    let env = s.nu(
        "def execute [data: record<a: int>]: nothing -> record<b: int> { {b: ($data.a + 1)} }",
        "{a: 41}",
    );
    assert!(!has_error(&env), "{env}");
    assert_eq!(env["result"], json!({"b": 42}));
    assert!(!nonce_of(&env).is_empty());
}

#[test]
fn a_table_round_trips() {
    let s = TESTING.harness();
    let env = s.nu(
        "def execute [data: table<n: int>]: nothing -> table<n: int, double: int> {\n    $data | each {|row| {n: $row.n, double: ($row.n * 2)} }\n}",
        "[[n]; [1] [2]]",
    );
    assert!(!has_error(&env), "{env}");
    assert_eq!(env["result"], json!([{"n": 1, "double": 2}, {"n": 2, "double": 4}]));
}

#[test]
fn a_nothing_definition_takes_null_and_answers_null() {
    let s = TESTING.harness();
    let env = s.nu("def execute [data: nothing]: nothing -> nothing { }", "null");
    assert!(!has_error(&env), "{env}");
    assert_eq!(env["result"], Value::Null);
}

#[test]
fn the_stateful_lane_keeps_env_and_cwd_between_calls() {
    let s = TESTING.harness();
    let set = s.nu(
        "def --env execute [data: record<v: string>]: nothing -> nothing { $env.NUNU_PROBE = $data.v }",
        "{v: kept}",
    );
    assert!(!has_error(&set), "{set}");
    let get = s.nu(
        "def --env execute [data: nothing]: nothing -> record<v: string> { {v: $env.NUNU_PROBE} }",
        "null",
    );
    assert_eq!(get["result"], json!({"v": "kept"}), "{get}");
}

#[test]
fn the_stateless_pool_keeps_nothing_between_calls() {
    let s = TESTING.harness();
    let set = s.nu(
        "def execute [data: nothing]: nothing -> nothing { $env.NUNU_LOST = 'yes' }",
        "null",
    );
    assert!(!has_error(&set), "{set}");
    let get = s.nu(
        "def execute [data: nothing]: nothing -> record<seen: bool> { {seen: ('NUNU_LOST' in $env)} }",
        "null",
    );
    assert_eq!(get["result"], json!({"seen": false}), "{get}");
}

#[test]
fn undeclared_data_is_refused_before_dispatch_with_a_nonce() {
    let s = TESTING.harness();
    let env = s.nu(
        "def execute [data: record<a: int>]: nothing -> record<a: int> { $data }",
        "{a: 1, b: 2}",
    );
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["data.b: an undeclared field".to_string()]);
    assert!(!nonce_of(&env).is_empty());
}

#[test]
fn data_that_is_not_nuon_is_refused() {
    let s = TESTING.harness();
    let env = s.nu("def execute [data: record<a: int>]: nothing -> record<a: int> { $data }", "{a: ");
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
    let env = s.nu("def execute [data: nothing]: nothing -> nothing { }", "");
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
}

#[test]
fn an_undeclared_result_field_is_refused_with_the_nonce() {
    let s = TESTING.harness();
    let env = s.nu(
        "def execute [data: nothing]: nothing -> record<a: int> { {a: 1, b: 2} }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("result::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["result.b: an undeclared field".to_string()]);
}

#[test]
fn a_result_of_the_wrong_shape_is_refused_when_it_runs() {
    let s = TESTING.harness();
    let env = s.nu(
        "def execute [data: nothing]: nothing -> record<a: int> { '[1]' | from nuon }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("result::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["result: expected record, got list<int>".to_string()]);
}

#[test]
fn a_statically_wrong_result_is_refused_at_parse() {
    let s = TESTING.harness();
    let env = s.nu(
        "def execute [data: record<n: int>]: nothing -> record<a: int> {\n    let rows = [$data.n]\n    $rows\n}",
        "{n: 1}",
    );
    assert_eq!(error_kind(&env), Some("definition::invalid"), "{env}");
}

#[test]
fn a_contract_violation_is_refused_as_a_definition_row() {
    let s = TESTING.harness();
    let env = s.nu("def execute [data: list<int>]: nothing -> nothing { }", "[1]");
    assert_eq!(error_kind(&env), Some("definition::invalid"), "{env}");
}

#[test]
fn the_timeout_reaches_a_body_that_declares_it() {
    let s = TESTING.harness();
    let env = s.nu_timeout(
        "def execute [data: nothing, timeout?: duration]: nothing -> record<within: bool> {\n    {within: ($timeout == 30sec)}\n}",
        "null",
        Some("30sec"),
    );
    assert_eq!(env["result"], json!({"within": true}), "{env}");
    let env = s.nu(
        "def execute [data: nothing, timeout?: duration]: nothing -> record<within: bool> {\n    {within: ($timeout == 120sec)}\n}",
        "null",
    );
    assert_eq!(env["result"], json!({"within": true}), "the default reaches it; {env}");
}

#[test]
fn a_timeout_fires() {
    let s = TESTING.harness();
    let env = s.nu_timeout(
        "def execute [data: nothing]: nothing -> nothing { sleep 10sec }",
        "null",
        Some("300ms"),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "{env}");
}

#[test]
fn a_bad_timeout_is_refused() {
    let s = TESTING.harness();
    for timeout in ["5", "soon", "-1sec", "0sec"] {
        let env = s.nu_timeout("def execute [data: nothing]: nothing -> nothing { }", "null", Some(timeout));
        assert_eq!(error_kind(&env), Some("timeout::invalid"), "{timeout}: {env}");
    }
}

#[test]
fn rerun_evaluates_a_cached_definition_with_fresh_data() {
    let s = TESTING.harness();
    let first = s.nu(
        "def execute [data: record<a: int>]: nothing -> record<b: int> { {b: ($data.a * 10)} }",
        "{a: 1}",
    );
    assert_eq!(first["result"], json!({"b": 10}), "{first}");
    let source = nonce_of(&first);
    let again = s.rerun(&source, "{a: 7}");
    assert_eq!(again["result"], json!({"b": 70}), "{again}");
    let fresh = nonce_of(&again);
    assert_ne!(fresh, source, "a rerun mints its own nonce");
    let chained = s.rerun(&fresh, "{a: 2}");
    assert_eq!(chained["result"], json!({"b": 20}), "a rerun's nonce is a handle too; {chained}");
    let refused = s.rerun(&source, "{a: x}");
    assert_eq!(error_kind(&refused), Some("data::invalid"), "{refused}");
}

#[test]
fn rerun_refuses_a_missing_or_malformed_nonce() {
    let s = TESTING.harness();
    let env = s.rerun("NoSuchNonce1", "null");
    assert_eq!(error_kind(&env), Some("rerun::body_missing"), "{env}");
    let env = s.rerun("../escape", "null");
    assert_eq!(error_kind(&env), Some("rerun::invalid_nonce"), "{env}");
}
