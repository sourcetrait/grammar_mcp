use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// initialize -> tools/list (exactly 21) -> a nu() round-trip. The rmcp stdio
/// transport + handshake are the system surface here.
#[tested]
fn tools_list_and_nu_round_trip() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());

    let names = host.tool_names();
    assert_eq!(names.len(), 21, "expected 21 tools; got {names:?}");
    for expected in [
        "nu",
        "rerun",
        "call",
        "processes",
        "kill",
        "info",
        "new",
        "commit",
        "equip",
        "inspect",
        "channel_open",
        "channel_verified",
        "channel_close",
        "config_channel",
        "purviews",
        "purview_configure",
        "purview_extend",
        "purview",
        "remote_channel_open",
        "remote_channel_close",
        "remote_channels",
    ] {
        assert!(
            names.contains(&expected.to_string()),
            "missing `{expected}` in {names:?}",
        );
    }

    let resp = host.nu(
        "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
        "{x: 5}",
    );
    let env = envelope(&resp);
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    assert!(
        !nonce.is_empty() && nonce.chars().all(|c| c.is_ascii_alphanumeric()),
        "nonce should be base62; got {nonce:?}",
    );
    assert_eq!(env["result"], json!({"out": 6}), "got {env}");
}
