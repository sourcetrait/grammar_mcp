//! The host's startup held to nushell's own on a pinned env: `$nu`'s paths, the
//! startup env and constants, std, and `print`, through run, interact, and call.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nu_config::{CliOverrides, NushellConfigDirs, TestEnv, resolve_paths};
use serde_json::{Value, json};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

const ID: &str = "parity";
const NAMESPACE: &str = "default";

/// The parent shell's `NU_VERSION`, which the host must not inherit.
const PARENT_NU_VERSION: &str = "0.0.0-parent";

/// Every `$nu` path field, read by a body.
const NU_PATHS: &str = "{
    home_dir: $nu.home-dir,
    default_config_dir: $nu.default-config-dir,
    config_path: $nu.config-path,
    env_path: $nu.env-path,
    history_path: $nu.history-path,
    loginshell_path: $nu.loginshell-path,
    plugin_path: $nu.plugin-path,
    data_dir: $nu.data-dir,
    cache_dir: $nu.cache-dir,
    vendor_autoload_dirs: $nu.vendor-autoload-dirs,
    user_autoload_dirs: $nu.user-autoload-dirs,
}";

/// The startup env and constants nushell sets, read by a body.
const STARTUP_ENV: &str = "{
    config: ($env.config | describe | str starts-with 'record<'),
    env_conversions: ($env.ENV_CONVERSIONS | describe),
    path: ($env.PATH | describe),
    nu_lib_dirs_env: $env.NU_LIB_DIRS,
    nu_lib_dirs_const: $NU_LIB_DIRS,
    nu_plugin_dirs_env: $env.NU_PLUGIN_DIRS,
    nu_plugin_dirs_const: $NU_PLUGIN_DIRS,
    nu_version: $env.NU_VERSION,
}";

/// A host on a pinned env, and what nushell resolves for that env.
struct Parity {
    host: Host,
    dirs: NushellConfigDirs,
    namespace: PathBuf,
    cache_namespace: PathBuf,
    host_stderr: PathBuf,
}

impl Parity {
    fn spawn(root: &Path) -> Self {
        let home = root.join("home");
        let config_home = root.join("config");
        let plugins = root.join("plugins");
        for dir in [&home, &config_home, &plugins] {
            std::fs::create_dir_all(dir).expect("mkdir a pinned dir");
        }
        let data_dirs = format!(
            "{}:{}",
            path_str(&root.join("share_a")),
            path_str(&root.join("share_b")),
        );
        let vendor = root.join("vendor");
        let host_stderr = root.join("host_stderr.log");
        let pinned = [
            ("HOME", path_str(&home)),
            ("XDG_CONFIG_HOME", path_str(&config_home)),
            ("XDG_DATA_DIRS", data_dirs.clone()),
            ("NU_VENDOR_AUTOLOAD_DIR", path_str(&vendor)),
            ("NU_VERSION", PARENT_NU_VERSION.to_string()),
            ("NU_LIB_DIRS", path_str(&root.join("parent_lib"))),
            ("GRAMMAR_MCP_PLUGIN_DIR", path_str(&plugins)),
        ];
        let envs: Vec<(&str, &str)> = pinned.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let host = Host::spawn_logged(
            &root.join("scratch"),
            &["--id", ID, "--namespace", NAMESPACE],
            &envs,
            &host_stderr,
        );
        let namespace = namespace_dir(host.data_home(), ID, NAMESPACE);
        let cache_namespace = namespace_dir(host.cache_home(), ID, NAMESPACE);
        let vars = HashMap::from([
            ("XDG_CONFIG_HOME".to_string(), path_str(&config_home)),
            ("XDG_DATA_HOME".to_string(), path_str(host.data_home())),
            ("XDG_CACHE_HOME".to_string(), path_str(host.cache_home())),
            ("XDG_DATA_DIRS".to_string(), data_dirs),
            ("NU_VENDOR_AUTOLOAD_DIR".to_string(), path_str(&vendor)),
        ]);
        let overrides = CliOverrides {
            plugin_file: Some(namespace.join("plugin.msgpackz")),
            ..Default::default()
        };
        let (dirs, _) = resolve_paths(&TestEnv::new(vars).with_home_dir(home), &overrides)
            .expect("nushell resolves the pinned env");
        Self {
            host,
            dirs,
            namespace,
            cache_namespace,
            host_stderr,
        }
    }

    /// The clean envelope of `tool` running `body`, panicking on an error.
    fn eval(&mut self, tool: &str, result_schema: Value, body: &str) -> Value {
        let resp = self.host.call(
            tool,
            json!({"args_schema": {}, "result_schema": result_schema, "args": {}, "body": body}),
        );
        extract_envelope(&resp).unwrap_or_else(|| panic!("{tool} should succeed; got {resp}"))
    }

    /// The stderr log of the call whose envelope is `env`, under `kind`.
    fn call_stderr(&self, kind: &str, env: &Value) -> String {
        let nonce = env["nonce"].as_str().expect("envelope nonce");
        let log = self.cache_namespace.join(kind).join(nonce).join("stderr");
        std::fs::read_to_string(&log).unwrap_or_else(|e| panic!("read {}: {e}", log.display()))
    }

    /// The host's own stderr so far.
    fn host_stderr(&self) -> String {
        std::fs::read_to_string(&self.host_stderr).unwrap_or_default()
    }

    /// The engine version the host was built against, from `info()`.
    fn engine_version(&mut self) -> String {
        let info = self.host.call("info", json!({}));
        structured(&info)["nu_version"]
            .as_str()
            .expect("info's nu_version")
            .to_string()
    }

    /// `$NU_PLUGIN_DIRS` as nushell sets it: config home's `plugins`, the exe's dir.
    fn plugin_dirs(&self) -> Value {
        let exe = std::fs::canonicalize(grammar_mcp_bin()).expect("canonicalize the host");
        let exe_dir = exe.parent().expect("the host's dir");
        json!([path_str(&self.dirs.config_home.join("plugins")), path_str(exe_dir)])
    }
}

/// `$nu`'s path fields as nushell renders them from `dirs`.
fn expected_nu(dirs: &NushellConfigDirs) -> Value {
    json!({
        "home_dir": shown(&dirs.home_dir),
        "default_config_dir": shown(&dirs.config_home),
        "config_path": shown(dirs.config_file.as_path()),
        "env_path": shown(dirs.env_file.as_path()),
        "history_path": shown(&dirs.config_home.join("history.txt")),
        "loginshell_path": shown(&dirs.config_home.join("login.nu")),
        "plugin_path": shown(dirs.plugin_file.as_path()),
        "data_dir": shown(&dirs.data_home),
        "cache_dir": shown(&dirs.cache_home),
        "vendor_autoload_dirs": listed(&dirs.vendor_autoload_dirs),
        "user_autoload_dirs": listed(&dirs.user_autoload_dirs),
    })
}

/// The schema of `NU_PATHS`'s record.
fn nu_paths_schema() -> Value {
    json!({
        "home_dir": "string",
        "default_config_dir": "string",
        "config_path": "string",
        "env_path": "string",
        "history_path": "string",
        "loginshell_path": "string",
        "plugin_path": "string",
        "data_dir": "string",
        "cache_dir": "string",
        "vendor_autoload_dirs": ["string"],
        "user_autoload_dirs": ["string"],
    })
}

/// A path as `$nu` shows it: canonical where it exists, as given otherwise.
fn shown(path: &Path) -> String {
    if path.exists() {
        path_str(&std::fs::canonicalize(path).expect("canonicalize"))
    } else {
        path_str(path)
    }
}

/// A list of paths as `$nu` shows it, each as given.
fn listed(paths: &[PathBuf]) -> Value {
    Value::Array(paths.iter().map(|p| Value::String(path_str(p))).collect())
}

fn path_str(path: &Path) -> String {
    path.to_str().expect("a UTF-8 path").to_string()
}

/// A body's `$nu` paths are nushell's for the same env.
#[tested]
fn run_sees_nushells_paths() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let env = p.eval("run", nu_paths_schema(), NU_PATHS);
    assert_eq!(env["result"], expected_nu(&p.dirs), "the body's $nu paths; got {env}");
}

/// A body sees the env and constants nushell's startup sets, the host's lib
/// dirs in both `NU_LIB_DIRS`, and the engine's `NU_VERSION`, not the parent's.
#[tested]
fn run_sees_nushells_startup_env() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let schema = json!({
        "config": "bool",
        "env_conversions": "string",
        "path": "string",
        "nu_lib_dirs_env": ["string"],
        "nu_lib_dirs_const": ["string"],
        "nu_plugin_dirs_env": ["string"],
        "nu_plugin_dirs_const": ["string"],
        "nu_version": "string",
    });
    let env = p.eval("run", schema, STARTUP_ENV);
    let lib_dirs = json!([path_str(&p.namespace)]);
    let expected = json!({
        "config": true,
        "env_conversions": "record",
        "path": "list<string>",
        "nu_lib_dirs_env": lib_dirs,
        "nu_lib_dirs_const": lib_dirs,
        "nu_plugin_dirs_env": [],
        "nu_plugin_dirs_const": p.plugin_dirs(),
        "nu_version": p.engine_version(),
    });
    assert_eq!(env["result"], expected, "the body's startup env; got {env}");
    assert_ne!(env["result"]["nu_version"], PARENT_NU_VERSION, "NU_VERSION is the engine's");
}

/// A body imports and runs nushell's standard library.
#[tested]
fn run_loads_the_standard_library() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let body = "use std/assert\nuse std/iter\n\
                let found = ([1 2 3] | iter find {|x| $x > 1 })\n\
                assert equal $found 2\n{ found: $found }";
    let env = p.eval("run", json!({"found": "int"}), body);
    assert_eq!(env["result"]["found"], 2, "std ran; got {env}");
}

/// A parse-time constant reads `$nu.home-dir` through the lint and the eval.
#[tested]
fn a_const_reads_the_home_at_parse_time() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let env = p.eval("run", json!({"home": "string"}), "const home = $nu.home-dir\n{ home: $home }");
    assert_eq!(env["result"]["home"], shown(&p.dirs.home_dir), "the const's home; got {env}");
}

/// `print` writes to the call's stderr log, honouring `--no-newline`, and
/// nothing of it reaches the host's own stderr.
#[tested]
fn print_lands_in_the_calls_stderr_log() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let body = "print 'PRINTMARK run'\n42 | print\nprint --no-newline 'PRINTMARK tail'\n{ ok: true }";
    let env = p.eval("run", json!({"ok": "bool"}), body);
    assert_eq!(p.call_stderr("runs", &env), "PRINTMARK run\n42\nPRINTMARK tail", "the call's log");
    assert!(!p.host_stderr().contains("PRINTMARK"), "the host's stderr: {}", p.host_stderr());
}

/// An interact body sees the same startup and its `print` lands in its log.
#[tested]
fn interact_sees_the_same_startup() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let body = "print 'PRINTMARK interact'\n\
                { home: $nu.home-dir, path: ($env.PATH | describe), \
                config: ($env.config | describe | str starts-with 'record<') }";
    let schema = json!({"home": "string", "path": "string", "config": "bool"});
    let env = p.eval("interact", schema, body);
    let expected = json!({"home": shown(&p.dirs.home_dir), "path": "list<string>", "config": true});
    assert_eq!(env["result"], expected, "the interact body's startup; got {env}");
    assert_eq!(p.call_stderr("interacts", &env), "PRINTMARK interact\n", "the call's log");
}

/// With the standard library's files ahead of a rig's in the span space, the
/// validator still reads each token: a `main` variable is refused at its line.
#[tested]
fn a_reserved_main_is_still_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let (_, src) = p.host.equip_new("sourcetrait/reserved", &t.temp_dir().join("src"));
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module probe\nexport use probe\n");
    write_source(
        &src,
        "m/probe/mod.nu",
        "export def main [args: record<noop: int>]: nothing -> record<n: int> {\n\
         let main = 1\n\
         { n: $main }\n\
         }\n",
    );
    let committed = p.host.commit("sourcetrait/reserved");
    let reserved: Vec<&Value> = envelope_error(&committed)
        .and_then(|e| e["errors"].as_array())
        .map(|errors| errors.iter().filter(|e| e["kind"] == "rig::reserved").collect())
        .unwrap_or_default();
    assert_eq!(reserved.len(), 1, "one reserved `main`; got {committed}");
    assert_eq!(reserved[0]["source"]["position"], json!([2, 0]), "at its line; got {committed}");
}

/// A rig reading `$nu.home-dir` in a constant and using std passes the
/// validator, and its call sees the same startup and logs its `print`.
#[tested]
fn a_rigs_call_sees_the_same_startup() {
    let t = testing::test!({ .using_temp_dir() });
    let mut p = Parity::spawn(t.temp_dir());
    let (_, src) = p.host.equip_new("sourcetrait/parity", &t.temp_dir().join("src"));
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module probe\nexport use probe\n");
    write_source(
        &src,
        "m/probe/mod.nu",
        "use std/assert\n\
         const HOME = $nu.home-dir\n\
         export def main [args: record<noop: int>]: nothing -> \
         record<home: string, path: string, plugin_dirs: list<string>> {\n\
         assert equal 1 1\n\
         print 'PRINTMARK call'\n\
         { home: $HOME, path: ($env.PATH | describe), plugin_dirs: $NU_PLUGIN_DIRS }\n\
         }\n",
    );
    let committed = p.host.commit("sourcetrait/parity");
    assert!(!has_error_path(&committed), "the validator takes the rig; got {committed}");
    let resp = p.host.call_np("sourcetrait/parity:m:probe", json!({"noop": 0}));
    let env = extract_envelope(&resp).unwrap_or_else(|| panic!("the call should succeed; got {resp}"));
    let expected = json!({
        "home": shown(&p.dirs.home_dir),
        "path": "list<string>",
        "plugin_dirs": p.plugin_dirs(),
    });
    assert_eq!(env["result"], expected, "the call's startup; got {env}");
    assert_eq!(p.call_stderr("calls", &env), "PRINTMARK call\n", "the call's log");
}
