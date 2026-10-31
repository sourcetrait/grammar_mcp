//! The instance file, and the paths a worker's identity resolves to.
use crate::*;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

/// The instance file's schema, which every instance file is held to.
const INSTANCE_MODEL: &str = include_str!("../../../../include/config/grammard/Instance.nutype");

/// The most bytes a Unix socket's path holds, its terminator among them.
#[cfg(target_os = "linux")]
const SUN_PATH_MAX: usize = 108;

/// The most bytes a Unix socket's path holds, its terminator among them.
#[cfg(not(target_os = "linux"))]
const SUN_PATH_MAX: usize = 104;

/// The supervisor's four lines, as the instance file sets them.
#[derive(Debug, Clone, Copy, PartialEq, ser::Deserialize)]
pub struct SupervisorLines {
    pub cpu_warn_fraction: f64,
    pub ram_warn_fraction: f64,
    pub vram_warn_headroom_mib: u64,
    pub disk_warn_fraction: f64,
}

/// The bounds the worker offers each hello, as the instance file sets them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ser::Deserialize)]
pub struct LimitLines {
    pub rpc_frame: u32,
    pub decode_heap: u64,
    pub concurrent_rpcs: u32,
    pub control_slots: u32,
    pub event_records: u32,
    pub event_bytes: u64,
    pub streams: u32,
}

/// The instance file as written: whose worker it is and how that worker runs.
#[derive(Debug, Clone, PartialEq, ser::Deserialize)]
pub struct InstanceFile {
    pub id: String,
    pub namespace: String,
    pub key: String,
    pub workdir: String,
    pub peer_config: Option<String>,
    pub supervisor: Option<SupervisorLines>,
    pub limits: Option<LimitLines>,
}

/// Where a worker's files are, each from its identity and its homes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstancePaths {
    /// The instance file.
    pub file: PathBuf,
    /// The worker's durable state.
    pub state: PathBuf,
    /// The engine's data home, the MCP's own layout.
    pub data: PathBuf,
    /// The engine's cache home, the MCP's own layout.
    pub cache: PathBuf,
    /// The engine socket, none without a runtime home.
    pub socket: Option<PathBuf>,
}

impl InstancePaths {
    /// The paths of `key`'s instance of `id` and `namespace` under `homes`.
    pub fn of(
        homes: &InstanceHomes,
        id: &InstanceName,
        namespace: &InstanceName,
        key: &InstanceKey,
    ) -> DaemonResult<Self> {
        let grammard = |home: &Path| home.join("grammar").join("grammard");
        let mcp = |home: &Path| home.join("grammar").join("mcp").join(id.as_str()).join(namespace.as_str());
        let socket = match &homes.runtime {
            Some(runtime) => {
                let socket = runtime.join("grammard").join(format!("{key}.sock"));
                if socket.as_os_str().len() + 1 > SUN_PATH_MAX {
                    return Err(DaemonError::PathTooLong {
                        path: socket,
                        limit: SUN_PATH_MAX,
                    });
                }
                Some(socket)
            }
            None => None,
        };
        Ok(Self {
            file: grammard(&homes.config).join("instances").join(format!("{key}.nuon")),
            state: grammard(&homes.state).join(key.as_str()),
            data: mcp(&homes.data),
            cache: mcp(&homes.cache),
            socket,
        })
    }
}

/// A worker's resolved instance: its identity, key, homes, paths, and file.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceConfig {
    pub identity: RuntimeIdentity,
    pub key: InstanceKey,
    pub homes: InstanceHomes,
    pub paths: InstancePaths,
    pub workdir: PathBuf,
    pub file: InstanceFile,
}

impl InstanceConfig {
    /// `id` and `namespace`'s instance for `uid`, its homes from `env`, its file read.
    pub fn load(
        uid: u32,
        id: &str,
        namespace: &str,
        env: &BTreeMap<String, String>,
    ) -> DaemonResult<Self> {
        let id = InstanceName::parse("id", id)?;
        let namespace = InstanceName::parse("namespace", namespace)?;
        let key = InstanceKey::of(&id, &namespace);
        let homes = InstanceHomes::resolve(env)?;
        let paths = InstancePaths::of(&homes, &id, &namespace, &key)?;
        let file = read_instance_file(&paths.file, uid)?;
        let invalid = |reason: String| DaemonError::ConfigInvalid {
            path: paths.file.clone(),
            reason,
        };
        if file.id != id.as_str() || file.namespace != namespace.as_str() || file.key != key.as_str() {
            return Err(invalid(format!(
                "it names {}/{} under key {}, another instance",
                file.id, file.namespace, file.key,
            )));
        }
        let workdir = PathBuf::from(&file.workdir);
        if !workdir.is_absolute() || !workdir.is_dir() {
            return Err(invalid(format!("its workdir {} is no absolute directory", workdir.display())));
        }
        let identity = RuntimeIdentity {
            uid,
            id: id.as_str().to_string(),
            namespace: namespace.as_str().to_string(),
        };
        Ok(Self {
            identity,
            key,
            homes,
            paths,
            workdir,
            file,
        })
    }
}

/// The instance file at `path`, refused unless `uid` owns it, private and regular.
fn read_instance_file(
    path: &Path,
    uid: u32,
) -> DaemonResult<InstanceFile> {
    let unsafe_file = |reason: String| DaemonError::ConfigUnsafe {
        path: path.to_path_buf(),
        reason,
    };
    let invalid = |reason: String| DaemonError::ConfigInvalid {
        path: path.to_path_buf(),
        reason,
    };
    let opened = fs::OpenOptions::new()
        .read(true)
        .custom_flags((nix::fcntl::OFlag::O_NOFOLLOW | nix::fcntl::OFlag::O_NONBLOCK).bits())
        .open(path);
    let mut handle = match opened {
        Ok(handle) => handle,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(DaemonError::ConfigMissing { path: path.to_path_buf() });
        }
        Err(e) => return Err(unsafe_file(format!("it does not open as a plain file: {e}"))),
    };
    let meta = handle.metadata().map_err(|e| unsafe_file(e.to_string()))?;
    if !meta.file_type().is_file() {
        return Err(unsafe_file("it is not a regular file".to_string()));
    }
    if meta.uid() != uid {
        return Err(unsafe_file(format!("UID {} owns it, not {uid}", meta.uid())));
    }
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(unsafe_file(format!("its mode {mode:o} opens it past 0600")));
    }
    let mut text = String::new();
    handle.read_to_string(&mut text).map_err(|e| invalid(e.to_string()))?;
    let value = NuVal::from_nuon(&text).map_err(|e| invalid(e.to_string()))?;
    let model = NuTypeFile::from_text(INSTANCE_MODEL).map_err(|e| invalid(format!("the instance model: {e}")))?;
    model.ty.check(&value).map_err(|e| invalid(e.to_string()))?;
    nuval::from_nuval(value).map_err(|e| invalid(e.to_string()))
}
