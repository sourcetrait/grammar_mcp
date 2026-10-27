//! The `interface` tool end to end through the in-process harness: every
//! command reached through its parse and dispatch, answering as its tool did,
//! and the refusals the contract makes.

use serde_json::{Value, json};
use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{
    TestServer, equip_source, error_kind, error_messages, has_error, valid_function_source,
    write_source,
};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// A string as a Nu literal, single-quoted; the tests' paths hold no quote.
fn quoted(text: &str) -> String {
    assert!(!text.contains('\''), "{text}");
    format!("'{text}'")
}

/// A successful envelope's result, panicking on an error envelope.
fn result_of(env: &Value) -> &Value {
    assert!(!has_error(env), "got {env}");
    &env["result"]
}

#[tested]
fn every_command_answers_through_the_interface() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();

    let info = s.interface("interface info");
    assert_eq!(result_of(&info)["name"], json!("grammar"), "got {info}");
    assert!(info["nonce"].is_string(), "the interface call mints the nonce; got {info}");

    let listed = s.interface("interface process list");
    assert_eq!(result_of(&listed)["processes"], json!([]), "got {listed}");
    let killed = s.interface("interface process kill 'noSuchNonce1'");
    assert_eq!(result_of(&killed), &Value::Null, "got {killed}");

    let source = equip_source(t.temp_dir(), "sourcetrait/ifprobe");
    let source_text = quoted(&source.to_string_lossy());
    let created = s.interface(&format!("interface equip new {source_text}"));
    assert!(result_of(&created)["summary"]["created"].is_array(), "got {created}");
    let scaffolded = s.interface("interface equip scaffold ['sourcetrait/ifprobe:m:go']");
    assert!(result_of(&scaffolded)["created"].is_array(), "got {scaffolded}");
    write_source(
        &source.join("rig"),
        "m/go/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($args.x * 2) }"),
    );
    let committed = s.interface("interface equip commit 'sourcetrait/ifprobe'");
    assert!(result_of(&committed)["added"].is_array(), "got {committed}");
    assert_eq!(result_of(&committed)["warnings"], json!([]), "got {committed}");
    let called = s.call("sourcetrait/ifprobe:m:go", json!({"x": 21}));
    assert_eq!(result_of(&called)["out"], json!(42), "got {called}");
    let checked = s.interface(&format!("interface equip check {source_text}"));
    assert_eq!(result_of(&checked)["summary"]["ok"], json!(true), "got {checked}");
    let helped = s.interface("interface help 'sourcetrait/ifprobe:m:go'");
    assert_eq!(
        result_of(&helped)["doc"]["signature"],
        json!("sourcetrait/ifprobe:m:go <x:int> <out:int>"),
        "got {helped}",
    );

    let configured = s.interface("interface purview configure 'iface/probe' ['sourcetrait/ifprobe:']");
    assert!(result_of(&configured)["signatures"].is_string(), "got {configured}");
    let extended = s.interface("interface purview extend ['iface/probe']");
    assert_eq!(
        result_of(&extended)["current"],
        json!(["default", "iface/probe"]),
        "got {extended}",
    );
    let set = s.interface("interface purview set []");
    assert!(!has_error(&set), "got {set}");
    let purviews = s.interface("interface purview list");
    assert_eq!(result_of(&purviews), &s.purviews()["result"], "got {purviews}");

    let uninstalled = s.interface(&format!("interface equip uninstall {source_text}"));
    assert_eq!(result_of(&uninstalled), &json!({}), "got {uninstalled}");
    let installed = s.interface(&format!("interface equip install {source_text}"));
    assert!(result_of(&installed)["summary"]["added"].is_array(), "got {installed}");
    let _ = s.interface(&format!("interface equip uninstall {source_text}"));

    let config = s.interface("interface channel config");
    assert_eq!(result_of(&config), &s.config_channel(None, None, None, None)["result"]);
    let tuned = s.interface("interface channel config --spam-warn-rate 11");
    assert_eq!(result_of(&tuned)["spam_warn_rate"], json!(11), "got {tuned}");
    let _ = s.interface("interface channel config --spam-warn-rate 10");
    let ready = s.interface("interface channel ready");
    assert_eq!(error_kind(&ready), Some("channel::not_open"), "got {ready}");
    let closed = s.interface("interface channel close");
    assert_eq!(result_of(&closed), &Value::Null, "got {closed}");

    let remotes = s.interface("interface remote list");
    assert_eq!(
        result_of(&remotes),
        &json!({"channels": [], "listening": []}),
        "got {remotes}",
    );
    let opened = s.interface("interface remote open 'nopeer'");
    assert_eq!(error_kind(&opened), Some("remote::channel_unverified"), "got {opened}");
    let shut = s.interface("interface remote close 'nopeer'");
    assert_eq!(error_kind(&shut), Some("remote::not_open"), "got {shut}");
}

#[test]
fn a_refusal_names_the_contract_and_carries_the_nonce() {
    let s = TESTING.harness();
    for command in [
        "interface nosuch",
        "interface info | interface purview list",
        "interface info; interface purview list",
        "interface help $nu",
        "interface help (interface info)",
        "interface help",
        "interface channel config --spam-warn-rate -1",
        "ls",
    ] {
        let env = s.interface(command);
        assert_eq!(error_kind(&env), Some("interface::invalid"), "`{command}`: {env}");
        assert!(env["nonce"].is_string(), "`{command}`: {env}");
    }
}

#[test]
fn a_command_refusal_is_its_tools() {
    let s = TESTING.harness();
    let env = s.interface("interface help 'grammar:eye:nosuch'");
    assert!(has_error(&env), "got {env}");
    let unknown_equip = s.interface("interface equip commit 'acme/nosuch'");
    assert_eq!(error_kind(&unknown_equip), Some("equip::not_registered"), "got {unknown_equip}");
    assert_eq!(
        error_messages(&unknown_equip),
        vec!["equip `acme/nosuch` is not registered".to_string()],
    );
}
