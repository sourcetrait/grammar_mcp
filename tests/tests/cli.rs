use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

#[tested]
fn cli_info_prints_its_envelope() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "info"]);
    assert!(out.status.success(), "cli info should exit 0; got {out:?}");
    let v = stdout_envelope(&out);
    let info = &v["result"];
    assert_eq!(info["name"].as_str(), Some("grammar"), "got {v}");
    assert_eq!(info["id"].as_str(), Some("cid"), "got {v}");
    assert_eq!(info["namespace"].as_str(), Some("default"), "got {v}");
    assert!(
        info["nu_version"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "info should carry nu_version; got {v}",
    );
}

#[tested]
fn cli_info_starts_without_the_sourcetrait_environment() {
    // The embedded default cert_dir names $XDGX_SECRET_DATA_HOME; with the
    // family absent the expansion falls back to its spec defaults instead of
    // failing the load, so the host starts on a non-box system.
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output_without(
        t.temp_dir(),
        &["--id", "cid", "cli", "info"],
        &[
            "XDGX_ASSET_HOME",
            "XDGX_BASE_SPEC",
            "XDGX_EXECUTE_HOME",
            "XDGX_LIBRARY_HOME",
            "XDGX_PACKAGE_HOME",
            "XDGX_SECRET_DATA_HOME",
            "XDGX_SHM_DIR",
            "XDGX_TMP_HOME",
        ],
    );
    assert!(
        out.status.success(),
        "a host without the XDGX environment must start; got {out:?}",
    );
    let v = stdout_envelope(&out);
    assert_eq!(v["result"]["name"].as_str(), Some("grammar"), "got {v}");
}

#[tested]
fn cli_equip_lifecycle_and_call() {
    let t = testing::test!({ .using_temp_dir() });
    let equip = equip_source(&t.temp_dir().join("src"), "sourcetrait/clilib");
    let src = equip.join("rig");

    let established = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "equip", "new", equip.to_str().unwrap()],
    );
    assert!(
        established.status.success(),
        "cli equip new should exit 0; got {established:?}",
    );

    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use double\n");
    write_source(
        &src,
        "m/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($args.x * 2) }"),
    );

    let committed = run_output(t.temp_dir(), &["--id", "cid", "cli", "commit", "sourcetrait/clilib"]);
    assert!(
        committed.status.success(),
        "cli commit should exit 0; got stdout={:?} stderr={:?}",
        stdout_str(&committed),
        String::from_utf8_lossy(&committed.stderr),
    );

    let called = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "call", "sourcetrait/clilib:m:double", "{x: 21}"],
    );
    assert!(
        called.status.success(),
        "cli call should exit 0; got stdout={:?} stderr={:?}",
        stdout_str(&called),
        String::from_utf8_lossy(&called.stderr),
    );
    let v = stdout_envelope(&called);
    assert_eq!(v["result"]["out"].as_i64(), Some(42), "got {v}");
}

#[tested]
fn cli_nu_evaluates_a_definition_against_nuon_data() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &[
            "--id",
            "cid",
            "cli",
            "nu",
            "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
            "{x: 5}",
        ],
    );
    assert!(
        out.status.success(),
        "cli nu should exit 0; got stdout={:?} stderr={:?}",
        stdout_str(&out),
        String::from_utf8_lossy(&out.stderr),
    );
    let v = stdout_envelope(&out);
    assert_eq!(v["result"]["out"].as_i64(), Some(6), "got {v}");
    assert!(v["nonce"].as_str().is_some(), "the envelope should carry a nonce; got {v}");
}

#[tested]
fn cli_error_envelope_exits_one() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "call", "sourcetrait/ghost:m:noop", "{}"],
    );
    assert_eq!(out.status.code(), Some(1), "an error envelope should exit 1; got {out:?}");
    let v = stdout_envelope(&out);
    assert!(v.get("error").is_some(), "the error envelope should print; got {v}");
    assert!(v["nonce"].as_str().is_some(), "a failure carries its nonce too; got {v}");
}

#[tested]
fn cli_output_is_one_nuon_record() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "info"]);
    assert!(out.status.success(), "cli info should exit 0; got {out:?}");
    let text = stdout_str(&out);
    assert!(!text.contains('\u{1b}'), "stdout must carry no ANSI escapes; got {text:?}");
    let trimmed = text.trim();
    assert!(
        trimmed.starts_with("{nonce: ") && trimmed.ends_with('}'),
        "stdout should be exactly one NUON record led by its nonce; got {text:?}",
    );
    assert!(try_nuon_to_json(trimmed).is_some(), "and it parses as NUON; got {text:?}");
}

#[tested]
fn cli_kill_prints_a_null_result_and_exits_zero() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "kill", "doesnotexist"]);
    assert!(out.status.success(), "cli kill should exit 0; got {out:?}");
    let v = stdout_envelope(&out);
    assert!(
        v.get("result").is_some_and(|result| result.is_null()),
        "a void tool answers a null result; got {v}",
    );
}
