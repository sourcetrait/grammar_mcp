//! A test instance on disk, its homes and its files, and a worker started on it.
#![allow(dead_code, unused_imports)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sourcetrait_grammar_daemon::*;
use sourcetrait_grammar_protocol::LIMITS_DEFAULT;

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
        let instance = Self::homes(id);
        instance.add(namespace);
        instance
    }

    /// `id`'s homes made, `namespace`'s file offering `streams` streams at once.
    pub fn with_streams(id: &str, namespace: &str, streams: u32) -> Self {
        let instance = Self::homes(id);
        let d = LIMITS_DEFAULT;
        let limits = format!(
            "{{rpc_frame: {}, decode_heap: {}, concurrent_rpcs: {}, control_slots: {}, event_records: {}, event_bytes: {}, streams: {streams}}}",
            d.rpc_frame, d.decode_heap, d.concurrent_rpcs, d.control_slots, d.event_records, d.event_bytes,
        );
        instance.write(namespace, &limits);
        instance
    }

    /// `id`'s homes alone, each made, and the environment naming them.
    fn homes(id: &str) -> Self {
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
        Self { root, run, id: id.to_string(), env }
    }

    /// `namespace`'s instance file written beside the others, private to its owner.
    pub fn add(&self, namespace: &str) {
        self.write(namespace, "null");
    }

    /// `namespace`'s instance file with `limits` as its limits' NUON.
    fn write(&self, namespace: &str, limits: &str) {
        let name = |field, text| InstanceName::parse(field, text).expect("a name");
        let key = InstanceKey::of(&name("id", &self.id), &name("namespace", namespace));
        let dir = self.root.path().join("config/grammar/grammard/instances");
        std::fs::create_dir_all(&dir).expect("the instances directory");
        let file = dir.join(format!("{key}.nuon"));
        let text = format!(
            "{{id: {:?}, namespace: {namespace:?}, key: {:?}, workdir: {:?}, peer_config: null, supervisor: null, limits: {limits}}}",
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

#[cfg(feature = "service")]
pub use worker::*;

/// A grammard worker run as a child, never outliving its test.
#[cfg(feature = "service")]
mod worker {
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;
    use std::process::{Child, Command, ExitStatus, Output, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::TestInstance;

    /// How long a start may take to its ready line or its exit.
    const READY_DEADLINE: Duration = Duration::from_secs(60);

    /// How long a stopped worker may take to exit before it is killed.
    const STOP_DEADLINE: Duration = Duration::from_secs(20);

    /// A worker serving, killed at its drop unless already stopped.
    pub struct Worker {
        child: Option<Child>,
        pub generation: String,
        pub socket: PathBuf,
    }

    impl Worker {
        pub fn pid(&self) -> u32 {
            self.child.as_ref().map(Child::id).unwrap_or_default()
        }

        /// Stopped as a service manager stops it, killed past the deadline, its exit given.
        pub fn stop(mut self) -> ExitStatus {
            let mut child = self.child.take().expect("a running worker");
            let pid = nix::unistd::Pid::from_raw(child.id() as i32);
            nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM).expect("the stop");
            wait_within(&mut child, STOP_DEADLINE)
        }

        /// Killed outright, which runs no cleanup.
        pub fn kill(mut self) {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    impl Drop for Worker {
        fn drop(&mut self) {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    /// `child`'s exit within `deadline`, else its exit once killed.
    fn wait_within(child: &mut Child, deadline: Duration) -> ExitStatus {
        let started = Instant::now();
        loop {
            if let Ok(Some(status)) = child.try_wait() {
                return status;
            }
            if started.elapsed() > deadline {
                let _ = child.kill();
                return child.wait().expect("the killed worker's exit");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// `namespace`'s worker on `instance`'s environment, ready, or its failed output.
    pub fn start(instance: &TestInstance, namespace: &str) -> Result<Worker, Output> {
        start_with(instance, namespace, &instance.env)
    }

    /// `namespace`'s worker on `env`, ready within the deadline, or its failed output.
    pub fn start_with(instance: &TestInstance, namespace: &str, env: &BTreeMap<String, String>) -> Result<Worker, Output> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_grammard"))
            .args(["serve", "--id", &instance.id, "--namespace", namespace])
            .env_clear()
            .envs(env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("grammard runs");
        let mut stderr = child.stderr.take().unwrap();
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
            text
        });
        let stdout = child.stdout.take().unwrap();
        let (ready, readied) = mpsc::channel();
        std::thread::spawn(move || {
            let mut lines = BufReader::new(stdout).lines();
            let mut sent = false;
            while let Some(Ok(line)) = lines.next() {
                if !sent && line.contains("at generation ") {
                    let _ = ready.send(line);
                    sent = true;
                }
            }
        });
        match readied.recv_timeout(READY_DEADLINE) {
            Ok(line) => {
                let rest = line.split_once("at generation ").map(|(_, rest)| rest).unwrap_or_default();
                let generation = rest.split(',').next().unwrap_or_default().to_string();
                let socket = PathBuf::from(line.split_once(", socket ").map(|(_, path)| path).unwrap_or_default());
                Ok(Worker { child: Some(child), generation, socket })
            }
            Err(timed_out) => {
                if timed_out == mpsc::RecvTimeoutError::Timeout {
                    let _ = child.kill();
                }
                let status = wait_within(&mut child, STOP_DEADLINE);
                let stderr = errors.join().unwrap_or_default().into_bytes();
                Err(Output { status, stdout: Vec::new(), stderr })
            }
        }
    }
}
