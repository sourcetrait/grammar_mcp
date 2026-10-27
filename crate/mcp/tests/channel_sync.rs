//! A sync Channel through the interface: its handshake, a body's send read by
//! `interface channel sync`, and its refusals.
//!
//! A binary of its own, its cases one Stepper sequence: the Channel is the
//! process's one, so every case reads or changes the same state.

use serde_json::{Value, json};
use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{TestServer, error_kind, error_messages, has_error};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// The one packet a sync answered, every field the wire line carries.
fn one_packet(env: &Value) -> &Value {
    let packets = env["result"]["packets"]
        .as_array()
        .unwrap_or_else(|| panic!("a packet table; got {env}"));
    assert_eq!(packets.len(), 1, "{env}");
    assert_eq!(env["result"]["dropped"], json!(0), "{env}");
    &packets[0]
}

fn a_closed_channel_refuses_a_sync() {
    let s = TESTING.harness();
    let env = s.interface("interface channel sync");
    assert_eq!(error_kind(&env), Some("channel::not_open"), "{env}");
}

fn a_sync_open_binds_no_endpoint() {
    let s = TESTING.harness();
    let env = s.interface("interface channel open sync");
    assert!(!has_error(&env), "{env}");
    assert_eq!(env["result"]["status"], json!("new"), "{env}");
    assert_eq!(env["result"]["mode"], json!("sync"), "{env}");
    assert_eq!(env["result"]["wss"], Value::Null, "{env}");
    assert!(env["result"]["inbox"].is_string(), "{env}");
}

fn ready_waits_for_the_first_sync() {
    let s = TESTING.harness();
    let env = s.interface("interface channel ready");
    assert_eq!(error_kind(&env), Some("channel::not_claimed"), "{env}");
    assert!(error_messages(&env)[0].contains("interface channel sync"), "{env}");
}

fn the_first_sync_delivers_the_greeting() {
    let s = TESTING.harness();
    let env = s.interface("interface channel sync");
    let packet = one_packet(&env);
    assert_eq!(packet["from"], json!("mcp"), "{env}");
    assert_eq!(packet["model"], json!("mcp/channel/Open"), "{env}");
    assert!(packet["event"]["mcp_nom"].is_string(), "{env}");
    assert!(packet["id"].is_string(), "{env}");
    assert_eq!(packet["attached"], Value::Null, "{env}");
    let ready = s.interface("interface channel ready");
    assert!(!has_error(&ready), "the sync claimed the channel; {ready}");
}

fn a_body_send_waits_for_the_next_sync() {
    let s = TESTING.harness();
    let sent = s.execute(
        "def execute [data: nothing]: nothing -> record<ok: bool> {\n    \
         grimm channel send 'two/test/Ping' {n: 1} | ignore\n    {ok: true}\n}",
        "null",
    );
    assert!(!has_error(&sent), "{sent}");
    let nonce = sent["nonce"].as_str().expect("a nonce");
    let env = s.interface("interface channel sync");
    let packet = one_packet(&env);
    assert_eq!(packet["from"], json!(format!("thread/{nonce}")), "{env}");
    assert_eq!(packet["model"], json!("two/test/Ping"), "{env}");
    assert_eq!(packet["event"], json!({"n": 1}), "{env}");
    let again = s.interface("interface channel sync");
    assert_eq!(again["result"]["packets"], json!([]), "the sync emptied the queue; {again}");
}

fn the_other_mode_is_refused_while_open() {
    let s = TESTING.harness();
    let env = s.interface("interface channel open async");
    assert_eq!(error_kind(&env), Some("channel::wrong_mode"), "{env}");
    assert_eq!(
        error_messages(&env),
        vec![
            "the channel is open in `sync` mode, not `async`; close it with \
             `interface channel close` to open it in `async`"
                .to_string()
        ],
    );
}

fn reopening_reports_it_and_greets_again() {
    let s = TESTING.harness();
    let env = s.interface("interface channel open sync");
    assert_eq!(env["result"]["status"], json!("existing"), "{env}");
    assert_eq!(env["result"]["wss"], Value::Null, "{env}");
    let synced = s.interface("interface channel sync");
    assert_eq!(one_packet(&synced)["model"], json!("mcp/channel/Open"), "{synced}");
}

fn a_close_ends_it() {
    let s = TESTING.harness();
    let closed = s.interface("interface channel close");
    assert!(!has_error(&closed), "{closed}");
    let env = s.interface("interface channel sync");
    assert_eq!(error_kind(&env), Some("channel::not_open"), "{env}");
}

#[tested]
fn the_sync_channel_is_sequenced() {
    testing::Stepper::builder("sync_channel")
        .init(|_t, _o: ()| testing::StepState((), ()))
        .step("a_closed_channel_refuses_a_sync", |_t, s, p| {
            a_closed_channel_refuses_a_sync();
            testing::StepState(s, p)
        })
        .step("a_sync_open_binds_no_endpoint", |_t, s, p| {
            a_sync_open_binds_no_endpoint();
            testing::StepState(s, p)
        })
        .step("ready_waits_for_the_first_sync", |_t, s, p| {
            ready_waits_for_the_first_sync();
            testing::StepState(s, p)
        })
        .step("the_first_sync_delivers_the_greeting", |_t, s, p| {
            the_first_sync_delivers_the_greeting();
            testing::StepState(s, p)
        })
        .step("a_body_send_waits_for_the_next_sync", |_t, s, p| {
            a_body_send_waits_for_the_next_sync();
            testing::StepState(s, p)
        })
        .step("the_other_mode_is_refused_while_open", |_t, s, p| {
            the_other_mode_is_refused_while_open();
            testing::StepState(s, p)
        })
        .step("reopening_reports_it_and_greets_again", |_t, s, p| {
            reopening_reports_it_and_greets_again();
            testing::StepState(s, p)
        })
        .step("a_close_ends_it", |_t, s, p| {
            a_close_ends_it();
            testing::StepState(s, p)
        })
        .finalize()
        .run(TESTING.as_testable(), ());
}
