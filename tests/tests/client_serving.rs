//! Serving is a client in every build: the bare entry lists the four tools,
//! answers the Channel's frontend-owned commands itself and every runtime-owned
//! one `daemon::unavailable`, and makes no engine state. These run against the
//! binary the cohort built, so a client build proves serving without
//! embedded-cli and an embedded-cli build proves it with.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The kind a runtime-owned operation answers while no runtime is reachable.
const UNAVAILABLE: &str = "daemon::unavailable";

/// Each runtime-owned interface command, `help` by its two branches.
const RUNTIME_OWNED: &[&str] = &[
    "interface info",
    "interface help interface",
    "interface help 'sourcetrait/geo'",
    "interface process list",
    "interface process kill 'aaaaaaaaaaa'",
    "interface purview list",
    "interface purview set ['default']",
    "interface purview extend ['core']",
    "interface purview configure 'mine' ['sourcetrait/geo:']",
    "interface channel sync",
    "interface channel config",
    "interface remote channel list",
    "interface remote channel get 'peer'",
    "interface remote channel set 'peer' --address '127.0.0.1:1'",
    "interface remote channel unset 'peer'",
    "interface remote channel listen 'peer'",
    "interface remote channel connect 'peer'",
    "interface remote channel close 'peer'",
    "interface equip new 'equip/sourcetrait/geo'",
    "interface equip install 'equip/sourcetrait/geo'",
    "interface equip check 'equip/sourcetrait/geo'",
    "interface equip uninstall 'equip/sourcetrait/geo'",
    "interface equip scaffold ['sourcetrait/geo:shape:area']",
    "interface equip commit 'sourcetrait/geo'",
];

/// A definition a reachable runtime would answer.
const DEFINITION: &str =
    "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }";

/// The revision without `initialize` that Claude Code probes for.
const MODERN: &str = "2026-07-28";

/// The per-request metadata a modern client sends with every request.
fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientInfo": {"name": "grammar-system-tests", "version": "0.0.1"},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

/// The entries directly under `dir`, by name.
fn entries(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("a directory entry").file_name().to_string_lossy().into_owned())
        .collect()
}

/// The host made no engine or equip state: its data and cache homes stay empty.
fn assert_no_engine_state(host: &Host) {
    for home in [host.data_home(), host.cache_home()] {
        let made = entries(home);
        assert!(made.is_empty(), "serving made {made:?} under {}", home.display());
    }
}

/// A runtime-owned operation's answer: unavailable, in an envelope with its nonce.
fn assert_unavailable(answer: &Value, what: &str) {
    assert_eq!(
        envelope_error_kind(answer).as_deref(),
        Some(UNAVAILABLE),
        "{what} answers unavailable; got {answer}",
    );
    assert!(
        envelope(answer)["nonce"].as_str().is_some_and(|nonce| !nonce.is_empty()),
        "{what}'s envelope carries its nonce; got {answer}",
    );
}

#[tested]
fn serving_lists_the_four_tools() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_client(t.temp_dir(), &[]);
    let mut names = host.tool_names();
    names.sort();
    assert_eq!(names, ["call", "execute", "interface", "rerun"]);
}

#[tested]
fn a_denied_tool_is_withheld_by_the_client() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_client(t.temp_dir(), &["--deny", "execute"]);
    let mut names = host.tool_names();
    names.sort();
    assert_eq!(names, ["call", "interface", "rerun"]);
}

#[tested]
fn each_runtime_owned_tool_answers_unavailable() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_client(t.temp_dir(), &[]);
    let executed = host.execute(DEFINITION, "{x: 5}");
    assert_unavailable(&executed, "execute");
    let rerun = host.call("rerun", json!({"nonce": "aaaaaaaaaaa", "data": "{x: 6}"}));
    assert_unavailable(&rerun, "rerun");
    let called = host.call_np("sourcetrait/geo:shape:area", "{width: 3.0, height: 4.0}");
    assert_unavailable(&called, "call");
    assert_no_engine_state(&host);
}

#[tested]
fn each_runtime_owned_command_answers_unavailable() {
    let t = testing::test!({ .using_temp_dir() });
    let cwd = t.temp_dir().join("cwd");
    std::fs::create_dir_all(&cwd).expect("mkdir the host's working directory");
    let mut host = Host::spawn_client_in(&t.temp_dir().join("scratch"), &[], &cwd);
    for command in RUNTIME_OWNED {
        let answer = host.interface(command);
        assert_unavailable(&answer, command);
    }
    assert_no_engine_state(&host);
    let written = entries(&cwd);
    assert!(written.is_empty(), "serving wrote {written:?} into its working directory");
}

#[tested]
fn a_host_the_client_does_not_know_is_refused_first() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_client(t.temp_dir(), &[]);
    let answer = host.call(
        "execute",
        json!({"host": "elsewhere", "definition": DEFINITION, "data": "{x: 5}"}),
    );
    assert_eq!(envelope_error_kind(&answer).as_deref(), Some("host::unknown"), "got {answer}");
}

#[tested]
fn a_command_off_its_contract_is_refused_by_the_client() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_client(t.temp_dir(), &[]);
    let answer = host.interface("interface info | interface purview list");
    assert_eq!(
        envelope_error_kind(&answer).as_deref(),
        Some("interface::invalid"),
        "got {answer}",
    );
}

/// A deeply nested command meets nushell's parse on a worker sized for it: the
/// contract refuses it, and the host still serves.
#[tested]
fn a_nested_command_leaves_the_client_alive() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_client(t.temp_dir(), &[]);
    let depth = 200;
    let command = format!("interface help {}'x'{}", "(".repeat(depth), ")".repeat(depth));
    let id = host.request_interface(&command);
    let answer = host.try_read_id(id).unwrap_or_else(|| {
        let end = host
            .wait_for_exit(Duration::from_secs(5))
            .map(|status| describe_exit(&status))
            .unwrap_or_else(|| "kept running".to_string());
        panic!("no answer to a command nested {depth} deep: the host {end}");
    });
    assert_eq!(
        envelope_error_kind(&answer).as_deref(),
        Some("interface::invalid"),
        "got {answer}",
    );
    assert_eq!(host.tool_names().len(), 4, "the host still serves");
}

/// The Channel is the frontend's: the client opens it, a WSS consumer sees its
/// Open packet, `ready` verifies it, and `close` closes it.
#[tested]
fn the_channel_handshake_completes_on_the_client() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let keys = generate_keys("client", &root.join("channel"));
    write_host_config(&root.join("host"), &keys, "");
    let host = Host::spawn_client_in(&root.join("scratch"), &[], &root.join("host"));
    let mut consumer = Consumer::attach(host, &keys.ca);
    let closed = consumer.host.interface("interface channel close");
    assert!(!has_error_path(&closed), "the client closes its Channel; got {closed}");
    assert_no_engine_state(&consumer.host);
}

/// `--test` selects the test namespace and nothing else: the runtime's
/// operations still answer unavailable, and no state is made.
#[tested]
fn the_test_flag_selects_the_namespace_alone() {
    let t = testing::test!({ .using_temp_dir() });
    let log = t.temp_dir().join("host_stderr.log");
    let mut host =
        Host::spawn_client_uninitialized(&t.temp_dir().join("scratch"), &["--test"], &log);
    let reply = host.initialize();
    let result = &reply["result"];
    assert_eq!(result["serverInfo"]["title"].as_str(), Some("grammar (test)"), "got {reply}");
    assert!(
        result["instructions"].as_str().is_some_and(|text| text.contains("namespace `test`")),
        "the instructions name the namespace; got {reply}",
    );
    let executed = host.execute(DEFINITION, "{x: 5}");
    assert_unavailable(&executed, "execute under --test");
    assert_no_engine_state(&host);
}

/// Claude Code's first message reaches the client as it reached the engine-backed
/// server: the probe answered with the modern revision, `tools/list` and a call
/// served without `initialize`.
#[tested]
fn the_discover_probe_finds_the_client() {
    let t = testing::test!({ .using_temp_dir() });
    let log = t.temp_dir().join("host_stderr.log");
    let mut host = Host::spawn_client_uninitialized(&t.temp_dir().join("scratch"), &[], &log);
    host.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": {"_meta": modern_meta()},
    }));
    let answer = host.read_id(1);
    let versions = answer["result"]["supportedVersions"]
        .as_array()
        .unwrap_or_else(|| panic!("a DiscoverResult; got {answer}"));
    assert!(versions.iter().any(|v| v == MODERN), "{MODERN} offered; got {answer}");
    host.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {"_meta": modern_meta()},
    }));
    let tools = host.read_id(2);
    assert_eq!(tools["result"]["tools"].as_array().map(Vec::len), Some(4), "got {tools}");
    host.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "_meta": modern_meta(),
            "name": "execute",
            "arguments": hosted(execute_args(DEFINITION, "{x: 5}")),
        },
    }));
    assert_unavailable(&host.read_id(3), "execute after the probe");
    let stderr = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(!stderr.contains("panicked"), "no panic; the host's stderr: {stderr}");
}

/// A client build carries no local CLI, and its refusal names the build that does.
#[cfg(not(feature = "embedded-cli"))]
#[tested]
fn the_local_cli_is_absent_from_a_client_build() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["cli", "interface", "interface info"]);
    assert_eq!(out.status.code(), Some(2), "got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("embedded-cli"), "the refusal names the feature; got {stderr}");
    for home in ["data", "cache"] {
        let made = entries(&t.temp_dir().join(home));
        assert!(made.is_empty(), "the refusal made {made:?} under {home}");
    }
}

/// A client build carries no test driver.
#[cfg(not(feature = "embedded-cli"))]
#[tested]
fn the_test_driver_is_absent_from_a_client_build() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &[TEST_DRIVER]);
    assert_eq!(out.status.code(), Some(2), "got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unrecognized subcommand"), "got {stderr}");
}

/// The client and the test driver advertise the same four tools, word for word.
#[cfg(feature = "embedded-cli")]
#[tested]
fn the_client_and_the_test_driver_advertise_the_same_tools() {
    let t = testing::test!({ .using_temp_dir() });
    let by_name = |mut tools: Vec<Value>| {
        tools.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        tools
    };
    let client = by_name(Host::spawn_client(&t.temp_dir().join("client"), &[]).tools());
    let driver = by_name(Host::spawn(&t.temp_dir().join("driver")).tools());
    assert_eq!(client, driver);
}
