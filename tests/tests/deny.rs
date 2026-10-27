use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

#[tested]
fn deny_removes_tools_from_list() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "nu,rerun"]);
    let names = host.tool_names();
    assert_eq!(names.len(), 19, "21 - 2 denied = 19; got {names:?}");
    for absent in ["nu", "rerun"] {
        assert!(
            !names.contains(&absent.to_string()),
            "denied `{absent}` must be absent; got {names:?}",
        );
    }
    for present in [
        "call",
        "new",
        "commit",
        "equip",
        "channel_open",
        "channel_verified",
        "channel_close",
        "config_channel",
        "purviews",
        "purview_configure",
        "purview_extend",
        "purview",
        "info",
        "inspect",
        "processes",
        "kill",
        "remote_channel_open",
        "remote_channel_close",
        "remote_channels",
    ] {
        assert!(
            names.contains(&present.to_string()),
            "`{present}` should remain; got {names:?}",
        );
    }
}

#[tested]
fn denied_tool_call_fails_at_protocol_layer() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "nu"]);
    let resp = host.nu(
        "def execute [data: nothing]: nothing -> record<out: int> { { out: 1 } }",
        "null",
    );
    assert!(
        extract_envelope(&resp).is_none() && envelope(&resp).is_null(),
        "a denied tool must not execute, and answers no envelope at all; got {resp}",
    );
}

#[tested]
fn deny_full_set_leaves_core_four() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(
        t.temp_dir(),
        &[
            "--deny",
            "nu,rerun,call,new,commit,equip,channel_open,channel_verified,channel_close,config_channel,purviews,purview_configure,purview_extend,purview,remote_channel_open,remote_channel_close,remote_channels",
        ],
    );
    let mut names = host.tool_names();
    names.sort();
    assert_eq!(
        names,
        vec!["info", "inspect", "kill", "processes"],
        "the full deny set leaves exactly the core four",
    );
}

#[tested]
fn unknown_deny_token_fails_startup() {
    let t = testing::test!({ .using_temp_dir() });
    for token in ["bogus", "run", "interact"] {
        let out = run_output(t.temp_dir(), &["--deny", token]);
        assert!(
            !out.status.success(),
            "the unknown deny token `{token}` should fail startup; got {:?}",
            out.status,
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(token),
            "the clap error should name the bad token; got {stderr:?}",
        );
    }
}
