//! Cooperative cancellation: kill(nonce) actually stops an in-flight `nu` call. A
//! SYSTEM test - it needs a real client sending kill WHILE a call is in flight
//! (pipelined requests over the stdio transport). No test-hooks: nu `sleep` polls
//! Signals, so it bails on the cancel.

use std::time::{Duration, Instant};

use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The in-flight entries `processes()` lists.
fn in_flight(host: &mut Host) -> Vec<serde_json::Value> {
    let resp = host.call("processes", json!({}));
    envelope(&resp)["result"]["processes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Poll processes() until an in-flight `nu` call appears; return its nonce.
fn poll_inflight_nu_nonce(host: &mut Host, secs: u64) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        for e in in_flight(host) {
            if e.get("tool").and_then(|tl| tl.as_str()) == Some("nu")
                && let Some(n) = e.get("nonce").and_then(|n| n.as_str())
            {
                return Some(n.to_string());
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Poll processes() until the in-flight set drains (or `secs` elapse).
fn poll_until_drained(host: &mut Host, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        if in_flight(host).is_empty() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[tested]
fn kill_stops_an_in_flight_call() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    // Launch a long call without blocking on its response.
    let call_id = host.request(
        "nu",
        nu_args(
            "def execute [data: nothing]: nothing -> record<out: int> {\n    sleep 30sec\n    { out: 0 }\n}",
            "null",
        ),
    );
    // It should register as in-flight; grab its nonce.
    let nonce = poll_inflight_nu_nonce(&mut host, 5).expect("the call should register as in-flight");
    // Kill it; its own void response is ignored.
    let _ = host.request("kill", json!({"nonce": nonce}));
    // Read the call's own response: a killed call bails on the cancel and returns
    // an error, not a completed result.
    let resp = host.read_id(call_id);
    assert!(
        has_error_path(&resp),
        "a killed call should return an error, not complete; got {resp}",
    );
    // And the in-flight set drains.
    assert!(
        poll_until_drained(&mut host, 5),
        "processes() should drain after the kill",
    );
}
