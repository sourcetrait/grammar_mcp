//! The grammar builtin's rig: installed from this repository's `equip/`, each
//! call runs by its bare name on the bundled plugins.

use std::path::PathBuf;

use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

const PLUGIN_DIR_ENV: &str = "GRAMMAR_MCP_PLUGIN_DIR";

/// This repository's grammar equip, `equip/sourcetrait/grammar`.
fn grammar_equip() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the tests crate sits in the repository")
        .join("equip")
        .join("sourcetrait")
        .join("grammar")
}

/// Each grammar call runs by its bare name: a template directory soaked, and a
/// tree listed both ways.
#[tested]
fn each_grammar_call_runs_by_its_bare_name() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let plugin_dir = bundle_plugins(root, &[("grammar", "nu_plugin_eye"), ("grammar", "nu_plugin_soak")]);
    let mut host = Host::spawn_full(
        &root.join("scratch"),
        &["--id", "builtin", "--namespace", "default"],
        &[(PLUGIN_DIR_ENV, plugin_dir.to_str().expect("utf-8 path"))],
    );
    let installed = host.equip("install", grammar_equip().to_str().expect("utf-8 path"));
    assert!(!has_error_path(&installed), "the grammar equip installs; got {installed}");

    let info = structured(&host.call("info", json!({}))).clone();
    let signatures = info["signatures"].as_str().expect("info carries signatures");
    assert!(
        signatures.starts_with("grammar # "),
        "the builtin leads the block by its bare name; got:\n{signatures}",
    );

    let work = root.join("work");
    std::fs::create_dir_all(work.join(".git")).expect("mkdir the repository's .git");
    std::fs::write(work.join(".gitignore"), "*.log\n").expect("write the .gitignore");
    std::fs::write(work.join("a.log"), "log").expect("write the ignored file");
    let src = work.join("tmpl");
    std::fs::create_dir_all(&src).expect("mkdir the templates");
    std::fs::write(src.join("hello.txt.liquid"), "Hello, {{ name }}!").expect("write the template");
    let out = work.join("out");

    let soaked = host.call_np(
        "grammar:soak:dir",
        json!({"src": &src, "dst": &out, "fill": {"name": "World"}}),
    );
    assert!(!has_error_path(&soaked), "soak dir runs; got {soaked}");
    assert_eq!(
        std::fs::read_to_string(out.join("hello.txt")).expect("read the soaked output"),
        "Hello, World!",
    );

    let dir = host.call_np("grammar:eye:dir", json!({"dir": &work, "ignore": ["tmpl"], "regard": []}));
    let tree = structured(&dir)["result"]["tree"].as_str().expect("eye dir answers a tree");
    assert!(tree.contains(" a.log 3b"), "eye dir lists the ignored file; got {tree}");
    assert!(!tree.contains("hello.txt.liquid"), "eye dir skips an ignore glob; got {tree}");

    let git = host.call_np("grammar:eye:git", json!({"dir": &work}));
    let tree = structured(&git)["result"]["tree"].as_str().expect("eye git answers a tree");
    assert!(!tree.contains("a.log"), "eye git skips the ignored file; got {tree}");
    assert!(tree.contains("hello.txt.liquid"), "eye git lists the template; got {tree}");
}
