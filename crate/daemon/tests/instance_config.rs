//! An instance's names, key, homes, file, and socket path, each held or refused.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sourcetrait_grammar_daemon::*;

/// The worker's own effective UID, the instance file's owner in every test.
fn uid() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// A scratch tree of homes: each made, and the environment naming them.
struct Homes {
    root: tempfile::TempDir,
    /// The runtime home, short enough for a socket under macOS's 104-byte sockaddr.
    run: tempfile::TempDir,
    env: BTreeMap<String, String>,
}

impl Homes {
    /// Homes named by their UENV variables, every one made.
    fn new() -> Self {
        let root = tempfile::tempdir().expect("a scratch root");
        let run = tempfile::Builder::new().prefix("g").tempdir_in("/tmp").expect("a runtime home");
        let mut env = BTreeMap::new();
        for (var, dir) in [
            ("UENV_USR_CONFIG", "config"),
            ("UENV_USR_DATA", "data"),
            ("UENV_USR_CACHE", "cache"),
            ("UENV_USR_STATE", "state"),
            ("UENV_USR_TMP", "tmp"),
        ] {
            let path = root.path().join(dir);
            std::fs::create_dir_all(&path).expect("a home");
            env.insert(var.to_string(), path.to_string_lossy().into_owned());
        }
        env.insert("UENV_USR_TRANSIENT".to_string(), run.path().to_string_lossy().into_owned());
        env.insert("HOME".to_string(), root.path().to_string_lossy().into_owned());
        Self { root, run, env }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.path().join(rel)
    }
}

/// `text` as the instance file of `id` and `namespace` under `homes`, at `mode`.
fn write_instance(homes: &Homes, id: &str, namespace: &str, text: &str, mode: u32) -> PathBuf {
    let key = InstanceKey::of(&InstanceName::parse("id", id).unwrap(), &InstanceName::parse("namespace", namespace).unwrap());
    let dir = homes.path("config").join("grammar").join("grammard").join("instances");
    std::fs::create_dir_all(&dir).expect("the instances directory");
    let file = dir.join(format!("{key}.nuon"));
    std::fs::write(&file, text).expect("the instance file");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).expect("its mode");
    file
}

/// An instance file's text for `id` and `namespace`, its key and workdir its own.
fn instance_text(id: &str, namespace: &str, key: &str, workdir: &Path) -> String {
    format!(
        "{{id: {id:?}, namespace: {namespace:?}, key: {key:?}, workdir: {:?}, peer_config: null, supervisor: null, limits: null}}",
        workdir.to_string_lossy(),
    )
}

fn key_of(id: &str, namespace: &str) -> InstanceKey {
    InstanceKey::of(&InstanceName::parse("id", id).unwrap(), &InstanceName::parse("namespace", namespace).unwrap())
}

#[test]
fn a_name_is_refused_never_sanitized() {
    for good in ["two", "default", "a", "A9", "x.y_z-1", &"n".repeat(64)] {
        assert!(InstanceName::parse("id", good).is_ok(), "{good:?}");
    }
    let bad = ["", ".", "..", ".hidden", "-x", "_x", "a/b", "a\\b", "a\0b", "a b", "é", &"n".repeat(65), "../up"];
    for name in bad {
        let refused = InstanceName::parse("namespace", name).expect_err(name);
        assert_eq!(refused.kind().name(), "instance::name_invalid", "{name:?}");
    }
}

#[test]
fn the_key_is_the_length_prefixed_hash_and_never_aliases() {
    assert_eq!(
        key_of("two", "default").as_str(),
        "662f2ca65d725081c49554113d2506a04d554c98e6112d39530611fbdc408e85",
        "the SHA-256 of 00000003 'two' 00000007 'default'",
    );
    assert_eq!(key_of("two", "default"), key_of("two", "default"), "the same pair, the same key");
    assert_ne!(key_of("ab", "c"), key_of("a", "bc"), "the length prefixes keep a split apart");
    assert_ne!(key_of("two", "test"), key_of("test", "two"));
}

#[test]
fn a_uenv_home_comes_before_its_standard_variable_and_its_default() {
    let homes = Homes::new();
    let mut env = homes.env.clone();
    let xdg = homes.path("xdg-config");
    std::fs::create_dir_all(&xdg).unwrap();
    env.insert("XDG_CONFIG_HOME".to_string(), xdg.to_string_lossy().into_owned());
    assert_eq!(InstanceHomes::resolve(&env).unwrap().config, homes.path("config"), "UENV first");
    env.insert("UENV_USR_CONFIG".to_string(), String::new());
    let resolved = InstanceHomes::resolve(&env).unwrap();
    assert_eq!(resolved.config, xdg, "an empty UENV reads as unset");
    assert!(resolved.diagnostics.iter().any(|d| d == "UENV_USR_CONFIG is ignored, empty"), "and says so");
    env.insert("UENV_USR_CONFIG".to_string(), "relative/config".to_string());
    let resolved = InstanceHomes::resolve(&env).unwrap();
    assert_eq!(resolved.config, xdg, "a relative UENV is ignored");
    assert!(resolved.diagnostics.iter().any(|d| d.contains("UENV_USR_CONFIG")), "and says so");
    env.remove("UENV_USR_CONFIG");
    env.remove("XDG_CONFIG_HOME");
    std::fs::create_dir_all(homes.path(".config")).unwrap();
    assert_eq!(InstanceHomes::resolve(&env).unwrap().config, homes.path(".config"), "the default under HOME");
}

#[test]
fn a_missing_home_is_refused_and_never_made() {
    let homes = Homes::new();
    let mut env = homes.env.clone();
    let missing = homes.path("not-there");
    env.insert("UENV_USR_STATE".to_string(), missing.to_string_lossy().into_owned());
    let refused = InstanceHomes::resolve(&env).expect_err("a missing state home");
    assert_eq!(refused.kind().name(), "instance::home_unavailable");
    assert!(!missing.exists(), "the home was not made");
    let mut no_runtime = homes.env.clone();
    no_runtime.remove("UENV_USR_TRANSIENT");
    let resolved = InstanceHomes::resolve(&no_runtime).expect("every other home resolves");
    assert_eq!(resolved.runtime, None, "the runtime home has no default");
}

#[test]
fn the_instance_file_loads_once_its_owner_mode_and_identity_hold() {
    let homes = Homes::new();
    let workdir = homes.path("work");
    std::fs::create_dir_all(&workdir).unwrap();
    let key = key_of("two", "default");
    write_instance(&homes, "two", "default", &instance_text("two", "default", key.as_str(), &workdir), 0o600);
    let loaded = InstanceConfig::load(uid(), "two", "default", &homes.env).expect("a private file of the worker's own");
    assert_eq!(loaded.identity, RuntimeIdentity { uid: uid(), id: "two".to_string(), namespace: "default".to_string() });
    assert_eq!(loaded.key, key);
    assert_eq!(loaded.workdir, workdir);
    assert_eq!(loaded.paths.state, homes.path("state").join("grammar").join("grammard").join(key.as_str()));
    assert_eq!(loaded.paths.data, homes.path("data").join("grammar").join("mcp").join("two").join("default"));
    assert_eq!(loaded.paths.socket, Some(homes.run.path().join("grammard").join(format!("{key}.sock"))));
    let reloaded = InstanceConfig::load(uid(), "two", "default", &homes.env).unwrap();
    assert_eq!(loaded, reloaded, "the same file, the same instance");
}

#[test]
fn an_unsafe_missing_or_foreign_instance_file_is_refused() {
    let homes = Homes::new();
    let workdir = homes.path("work");
    std::fs::create_dir_all(&workdir).unwrap();
    let key = key_of("two", "default");
    let kind = |id: &str, ns: &str, uid: u32| InstanceConfig::load(uid, id, ns, &homes.env).expect_err("refused").kind().name();
    assert_eq!(kind("two", "default", uid()), "instance::config_missing");
    let file = write_instance(&homes, "two", "default", &instance_text("two", "default", key.as_str(), &workdir), 0o644);
    assert_eq!(kind("two", "default", uid()), "instance::config_unsafe", "readable by others");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(kind("two", "default", uid().wrapping_add(1)), "instance::config_unsafe", "another UID's");
    let target = homes.path("elsewhere.nuon");
    std::fs::rename(&file, &target).unwrap();
    std::os::unix::fs::symlink(&target, &file).unwrap();
    assert_eq!(kind("two", "default", uid()), "instance::config_unsafe", "a symlink");
    std::fs::remove_file(&file).unwrap();
    nix::unistd::mkfifo(&file, nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR).expect("a FIFO");
    let env = homes.env.clone();
    let (sent, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(InstanceConfig::load(uid(), "two", "default", &env).map(|_| ()).map_err(|e| e.kind().name()));
    });
    let fifo = received.recv_timeout(std::time::Duration::from_secs(10)).expect("the open waits on no writer");
    assert_eq!(fifo, Err("instance::config_unsafe"), "a FIFO");
    std::fs::remove_file(&file).unwrap();
    let other = key_of("two", "other");
    write_instance(&homes, "two", "default", &instance_text("two", "other", other.as_str(), &workdir), 0o600);
    assert_eq!(kind("two", "default", uid()), "instance::config_invalid", "another instance's settings");
    write_instance(&homes, "two", "default", "{id: two", 0o600);
    assert_eq!(kind("two", "default", uid()), "instance::config_invalid", "not NUON");
    write_instance(&homes, "two", "default", &format!("{{id: two, namespace: default, key: {:?}}}", key.as_str()), 0o600);
    assert_eq!(kind("two", "default", uid()), "instance::config_invalid", "not the instance's shape");
}

#[test]
fn a_socket_path_past_the_sockaddr_is_refused() {
    let homes = Homes::new();
    let mut env = homes.env.clone();
    let deep = homes.path(&"d".repeat(120));
    std::fs::create_dir_all(&deep).unwrap();
    env.insert("UENV_USR_TRANSIENT".to_string(), deep.to_string_lossy().into_owned());
    let resolved = InstanceHomes::resolve(&env).unwrap();
    let id = InstanceName::parse("id", "two").unwrap();
    let ns = InstanceName::parse("namespace", "default").unwrap();
    let refused = InstancePaths::of(&resolved, &id, &ns, &InstanceKey::of(&id, &ns)).expect_err("past the sockaddr");
    assert_eq!(refused.kind().name(), "instance::path_too_long");
}
