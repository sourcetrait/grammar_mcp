//! The MCP's own plugin registry: a host registers the plugins its plugin
//! directory bundles, lists them, and runs them, and it reads no other registry.

use std::path::{Path, PathBuf};

use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

const PLUGIN_DIR_ENV: &str = "GRAMMAR_MCP_PLUGIN_DIR";

/// A host whose plugin directory holds `nu_plugin_inc` registers it at startup,
/// lists it in `info()`, and runs `inc` in a body.
#[tested]
fn a_bundled_plugin_is_registered_listed_and_run() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let plugin_dir = bundle_inc(root);
    let mut host = Host::spawn_full(
        &root.join("scratch"),
        &["--id", "plug", "--namespace", "default"],
        &[(PLUGIN_DIR_ENV, plugin_dir.to_str().expect("utf-8 path"))],
    );

    assert_eq!(plugin_names(&mut host), vec!["inc".to_string()], "the bundled plugin is listed");
    let resp = host.run(json!({
        "args_schema": {},
        "result_schema": {"version": "string"},
        "args": {},
        "body": "{ version: ('1.2.3' | inc --patch) }",
    }));
    assert!(!has_error_path(&resp), "a body runs the bundled plugin; got {resp}");
    assert_eq!(
        structured(&resp)["result"]["version"].as_str(),
        Some("1.2.4"),
        "inc bumps the patch; got {resp}",
    );
}

/// The user's own registry is never read: a host whose plugin directory is
/// empty lists nothing, though its `XDG_CONFIG_HOME` holds a registry naming
/// `inc`.
#[tested]
fn the_user_registry_is_never_read() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();

    // A registry naming inc, written by a host that bundles it.
    let plugin_dir = bundle_inc(root);
    let first = Host::spawn_full(
        &root.join("scratch_a"),
        &["--id", "plug", "--namespace", "default"],
        &[(PLUGIN_DIR_ENV, plugin_dir.to_str().expect("utf-8 path"))],
    );
    let registry = namespace_dir(first.data_home(), "plug", "default").join("plugin.msgpackz");
    assert!(registry.is_file(), "the bundling host writes {}", registry.display());

    // That registry placed where the user's nushell keeps its own.
    let config_home = root.join("config");
    std::fs::create_dir_all(config_home.join("nushell")).expect("mkdir nushell config");
    std::fs::copy(&registry, config_home.join("nushell").join("plugin.msgpackz"))
        .expect("copy the registry");
    drop(first);

    let empty = root.join("empty_plugins");
    std::fs::create_dir_all(&empty).expect("mkdir an empty plugin dir");
    let mut host = Host::spawn_full(
        &root.join("scratch_b"),
        &["--id", "plug", "--namespace", "default"],
        &[
            (PLUGIN_DIR_ENV, empty.to_str().expect("utf-8 path")),
            ("XDG_CONFIG_HOME", config_home.to_str().expect("utf-8 path")),
        ],
    );
    assert!(plugin_names(&mut host).is_empty(), "no plugin comes from the user's registry");
}

/// The release's own plugins, soak and eye, register at startup, list in
/// `info()`, and run in a body: a template rendered both ways and a tree
/// listed both ways.
#[tested]
fn the_soak_and_eye_plugins_register_list_and_run() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let plugin_dir = bundle(root, &["nu_plugin_eye", "nu_plugin_soak"]);
    let mut host = Host::spawn_full(
        &root.join("scratch"),
        &["--id", "plug", "--namespace", "default"],
        &[(PLUGIN_DIR_ENV, plugin_dir.to_str().expect("utf-8 path"))],
    );

    let mut names = plugin_names(&mut host);
    names.sort();
    assert_eq!(names, vec!["eye".to_string(), "soak".to_string()], "the bundled plugins are listed");

    let work = root.join("work");
    std::fs::create_dir_all(work.join(".git")).expect("mkdir the repository's .git");
    std::fs::write(work.join(".gitignore"), "*.log\n").expect("write the .gitignore");
    std::fs::write(work.join("a.log"), "log").expect("write the ignored file");
    let resp = host.run(json!({
        "args_schema": {"root": "string"},
        "result_schema": {"rendered": "string", "soaked": "string", "dir": "string", "git": "string"},
        "args": {"root": work.to_str().expect("utf-8 path")},
        "body": "let src = ($args.root | path join 'tmpl')\n\
                 mkdir $src\n\
                 'Hello, {{ name }}!' | save ($src | path join 'hello.txt.liquid')\n\
                 let out = ($args.root | path join 'out')\n\
                 {name: World} | soak dir $src $out\n\
                 {\n\
                     rendered: ({name: World} | soak str 'Hello, {{ name }}!')\n\
                     soaked: (open ($out | path join 'hello.txt'))\n\
                     dir: (eye dir $args.root)\n\
                     git: (eye git $args.root)\n\
                 }",
    }));
    assert!(!has_error_path(&resp), "a body runs the bundled plugins; got {resp}");
    let result = &structured(&resp)["result"];
    assert_eq!(result["rendered"].as_str(), Some("Hello, World!"), "soak str renders; got {resp}");
    assert_eq!(result["soaked"].as_str(), Some("Hello, World!"), "soak dir renders; got {resp}");
    let dir = result["dir"].as_str().expect("eye dir renders a string");
    let git = result["git"].as_str().expect("eye git renders a string");
    assert!(dir.contains(" a.log 3b"), "eye dir lists the ignored file; got {dir}");
    assert!(!git.contains("a.log"), "eye git skips the ignored file; got {git}");
    assert!(git.contains("hello.txt.liquid"), "eye git lists the template; got {git}");
}

/// A bundled plugin replaced in place re-registers though it is dated at the
/// epoch, as an OptZip unpacks every file: the registry follows the file's
/// change, not its modification time.
#[tested]
fn a_replaced_plugin_re_registers_whatever_its_mtime() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let plugin_dir = bundle_inc(root);
    let scratch = root.join("scratch");
    let args = ["--id", "plug", "--namespace", "default"];
    let envs = [(PLUGIN_DIR_ENV, plugin_dir.to_str().expect("utf-8 path"))];

    let mut first = Host::spawn_full(&scratch, &args, &envs);
    assert_eq!(plugin_names(&mut first), vec!["inc".to_string()], "the bundled plugin is listed");
    let registry = namespace_dir(first.data_home(), "plug", "default").join("plugin.msgpackz");
    let registered = modified(&registry);
    drop(first);

    let inc = plugin_dir.join("nu_plugin_inc");
    std::fs::remove_file(&inc).expect("remove the bundled plugin");
    std::fs::copy(grammar_mcp_bin().with_file_name("nu_plugin_inc"), &inc).expect("bundle it again");
    std::fs::File::options()
        .write(true)
        .open(&inc)
        .and_then(|file| file.set_modified(std::time::UNIX_EPOCH))
        .expect("date the plugin at the epoch");

    let mut second = Host::spawn_full(&scratch, &args, &envs);
    assert_eq!(plugin_names(&mut second), vec!["inc".to_string()], "the replaced plugin is listed");
    assert!(modified(&registry) > registered, "the replaced plugin re-registers");
}

/// A file's modification time.
fn modified(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", path.display()))
}

/// A plugin directory holding the built `nu_plugin_inc`, the test host's sibling.
fn bundle_inc(root: &Path) -> PathBuf {
    bundle(root, &["nu_plugin_inc"])
}

/// A plugin directory holding the built `names`, the test host's siblings.
fn bundle(root: &Path, names: &[&str]) -> PathBuf {
    let dir = root.join("plugin");
    std::fs::create_dir_all(&dir).expect("mkdir the plugin dir");
    for name in names {
        let built = grammar_mcp_bin().with_file_name(name);
        std::fs::copy(&built, dir.join(name)).unwrap_or_else(|e| {
            panic!("copy {}: {e}; build it with `cargo build --bin {name}`", built.display())
        });
    }
    dir
}

/// The plugin names `info()` lists.
fn plugin_names(host: &mut Host) -> Vec<String> {
    let info = structured(&host.call("info", json!({}))).clone();
    info["plugins"]
        .as_array()
        .expect("info carries plugins")
        .iter()
        .filter_map(|entry| entry[0].as_str().map(String::from))
        .collect()
}
