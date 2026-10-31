//! A test instance on disk: its homes, its runtime home, and its instance files.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sourcetrait_grammar_daemon::*;

/// One id's instances under scratch homes, each namespace's file written.
pub struct TestInstance {
    pub root: tempfile::TempDir,
    /// The runtime home, short enough for a socket under macOS's 104-byte sockaddr.
    pub run: tempfile::TempDir,
    pub id: String,
    pub env: BTreeMap<String, String>,
}

impl TestInstance {
    /// `id`'s homes made, `namespace`'s instance file written into them.
    pub fn new(id: &str, namespace: &str) -> Self {
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
        std::fs::create_dir_all(root.path().join("work")).expect("the workdir");
        env.insert("UENV_USR_TRANSIENT".to_string(), run.path().to_string_lossy().into_owned());
        env.insert("HOME".to_string(), root.path().to_string_lossy().into_owned());
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }
        let instance = Self { root, run, id: id.to_string(), env };
        instance.add(namespace);
        instance
    }

    /// `namespace`'s instance file written beside the others, private to its owner.
    pub fn add(&self, namespace: &str) {
        let name = |field, text| InstanceName::parse(field, text).expect("a name");
        let key = InstanceKey::of(&name("id", &self.id), &name("namespace", namespace));
        let dir = self.root.path().join("config/grammar/grammard/instances");
        std::fs::create_dir_all(&dir).expect("the instances directory");
        let file = dir.join(format!("{key}.nuon"));
        let text = format!(
            "{{id: {:?}, namespace: {namespace:?}, key: {:?}, workdir: {:?}, peer_config: null, supervisor: null, limits: null}}",
            self.id,
            key.as_str(),
            self.root.path().join("work").to_string_lossy(),
        );
        std::fs::write(&file, text).expect("the instance file");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("its mode");
    }

    /// `namespace`'s instance loaded as this process's user.
    pub fn load(&self, namespace: &str) -> InstanceConfig {
        let uid = nix::unistd::geteuid().as_raw();
        InstanceConfig::load(uid, &self.id, namespace, &self.env).expect("the instance loads")
    }

    /// A home under the scratch root.
    pub fn home(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    /// The worker scratch directories in `base`.
    pub fn worker_dirs(base: &Path) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(base)
            .expect("the base")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with("grammard.")))
            .collect();
        dirs.sort();
        dirs
    }
}
