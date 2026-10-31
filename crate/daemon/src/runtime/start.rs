//! The runtime's start and shutdown, one runtime a process.
use crate::*;

/// Whether this process has started its runtime, which it does at most once.
static STARTED: AtomicBool = AtomicBool::new(false);

/// What a worker's lock record names its holder.
const LOCK_HOLDER: &str = "grammard";

/// The source of every random id the worker draws.
const URANDOM: &str = "/dev/urandom";

/// How long a shutdown waits for the socket's tasks to end.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// The threads the socket's tasks run on.
const SOCKET_THREADS: usize = 2;

/// A worker's immutable identity: its effective UID, MCP id, and namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdentity {
    /// The effective Unix UID the worker runs as.
    pub uid: u32,
    /// The MCP id owning the state namespace.
    pub id: String,
    /// The state namespace within the id.
    pub namespace: String,
}

/// The persistent worker's runtime, which its host starts and shuts down.
#[derive(Debug)]
pub struct Runtime {
    config: InstanceConfig,
    generation: WorkerGeneration,
    scratch: WorkerScratch,
    diagnostics: Vec<String>,
    serving: Option<Serving>,
    lock: NamespaceLock,
}

/// The engine socket's service: its listener, its tasks' runtime, and their stop.
#[derive(Debug)]
struct Serving {
    listener: EngineListener,
    tasks: tk::runtime::Runtime,
    stop: tk::sync::watch::Sender<bool>,
}

impl Runtime {
    /// Starts the process's one runtime on `config`: its lock, its scratch, its engine.
    pub fn start(config: InstanceConfig) -> DaemonResult<Self> {
        if STARTED.swap(true, Ordering::AcqRel) {
            return Err(DaemonError::AlreadyStarted);
        }
        let (lock, generation, scratch) = claim(&config).inspect_err(|_| STARTED.store(false, Ordering::Release))?;
        let mut diagnostics = scratch.diagnostics().to_vec();
        let serving = start_engine(&config, &scratch, &mut diagnostics).and_then(|()| match &config.paths.socket {
            Some(path) => serve(path, &config, generation).map(Some),
            None => Ok(None),
        });
        let serving = match serving {
            Ok(serving) => serving,
            Err(e) => {
                let _ = scratch.remove();
                return Err(e);
            }
        };
        Ok(Self {
            config,
            generation,
            scratch,
            diagnostics,
            serving,
            lock,
        })
    }

    /// The engine socket's path, where the instance names one.
    pub fn socket(&self) -> Option<&Path> {
        self.serving.as_ref().map(|serving| serving.listener.path())
    }

    /// The identity the runtime started under.
    pub fn identity(&self) -> &RuntimeIdentity {
        &self.config.identity
    }

    /// This start's generation, the lock's record of it.
    pub fn generation(&self) -> WorkerGeneration {
        self.generation
    }

    /// The worker's scratch directories.
    pub fn scratch(&self) -> &WorkerScratch {
        &self.scratch
    }

    /// What the start found to report: the scratch's and the substrate's.
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    /// What this build of the runtime can do.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities::NONE
    }

    /// Shuts the runtime down: its socket closed, its scratch removed, its lock last.
    pub fn shutdown(self) -> DaemonResult<()> {
        let Self {
            scratch,
            serving,
            lock,
            ..
        } = self;
        if let Some(Serving { listener, tasks, stop }) = serving {
            let _ = stop.send(true);
            tasks.shutdown_timeout(SHUTDOWN_GRACE);
            drop(listener);
        }
        let removed = scratch.remove();
        drop(lock);
        removed
    }
}

/// The engine socket bound at `path`, its streams admitted on tasks of their own.
fn serve(
    path: &Path,
    config: &InstanceConfig,
    generation: WorkerGeneration,
) -> DaemonResult<Serving> {
    let unavailable = |reason: String| DaemonError::SocketUnavailable {
        path: path.to_path_buf(),
        reason,
    };
    let mut listener = EngineListener::bind(path)?;
    let instance = Instance::new(&config.identity.id, &config.identity.namespace).map_err(|e| unavailable(format!("{e:?}")))?;
    let profile = WorkerProfile {
        uid: config.identity.uid,
        instance,
        generation,
        limits: limits_of(config.file.limits),
        artifacts: ARTIFACT_LIMITS_DEFAULT,
        capabilities: Capabilities::NONE,
    };
    let tasks = tk::runtime::Builder::new_multi_thread()
        .worker_threads(SOCKET_THREADS)
        .thread_name("grammard")
        .enable_all()
        .build()
        .map_err(|e| unavailable(format!("its tasks' runtime: {e}")))?;
    let bound = listener.take().ok_or_else(|| unavailable("its listener is gone".to_string()))?;
    let accepted = {
        let _entered = tasks.enter();
        tk::net::UnixListener::from_std(bound).map_err(|e| unavailable(e.to_string()))?
    };
    let (stop, stopped) = tk::sync::watch::channel(false);
    tasks.spawn(accept(accepted, Arc::new(Admission::new(profile)), stopped));
    Ok(Serving { listener, tasks, stop })
}

/// The bounds the worker offers each hello: the instance file's, else the defaults.
fn limits_of(lines: Option<LimitLines>) -> Limits {
    match lines {
        Some(lines) => Limits {
            rpc_frame: lines.rpc_frame,
            decode_heap: lines.decode_heap,
            concurrent_rpcs: lines.concurrent_rpcs,
            control_slots: lines.control_slots,
            event_records: lines.event_records,
            event_bytes: lines.event_bytes,
            streams: lines.streams,
        },
        None => LIMITS_DEFAULT,
    }
}

/// The lock taken, this start's generation recorded in it, and the scratch made.
fn claim(config: &InstanceConfig) -> DaemonResult<(NamespaceLock, WorkerGeneration, WorkerScratch)> {
    let unavailable = |reason: String| DaemonError::LockUnavailable {
        path: config.paths.lock.clone(),
        reason,
    };
    let mut lock = NamespaceLock::acquire(&config.paths.lock)?;
    let generation = fresh_generation().map_err(|e| unavailable(format!("no generation can be drawn: {e}")))?;
    let owner = Owner::current().map_err(|e| unavailable(e.to_string()))?;
    lock.record(&LockRecord {
        holder: LOCK_HOLDER,
        uid: config.identity.uid,
        id: config.identity.id.clone(),
        namespace: config.identity.namespace.clone(),
        pid: owner.pid,
        start: owner.start,
        boot: owner.boot,
        generation: generation.hex(),
    })?;
    let scratch = WorkerScratch::create(&config.homes)?;
    Ok((lock, generation, scratch))
}

/// The engine started once on the instance's homes and scratch, its substrate made.
fn start_engine(
    config: &InstanceConfig,
    scratch: &WorkerScratch,
    diagnostics: &mut Vec<String>,
) -> DaemonResult<()> {
    let failed = |reason: String| DaemonError::EngineInit { reason };
    let mcp_nom = datum::Nom::generate(&datum::NonceGenerator::new()).into_pair();
    engine::init(engine::EngineParams {
        id: config.identity.id.clone(),
        namespace: config.identity.namespace.clone(),
        work_dir: config.workdir.clone(),
        tmp_dir: scratch.tmp().to_path_buf(),
        transient_dir: scratch.transient().to_path_buf(),
        mcp_nom: mcp_nom.as_str().to_string(),
        data_home: config.paths.data.clone(),
        cache_home: config.paths.cache.clone(),
    })
    .map_err(|e| failed(e.to_string()))?;
    let fresh = engine::ensure_equip_store().map_err(|e| failed(format!("the equip store: {e}")))?;
    if let Err(e) = engine::ensure_default_purview() {
        diagnostics.push(format!("the default purview is not made: {e:?}"));
    }
    engine::ensure_plugin_registry();
    if fresh {
        engine::seed_builtin_purviews(&engine::install_builtins());
    }
    Ok(())
}

/// Sixteen random bytes, a generation no earlier start holds.
fn fresh_generation() -> io::Result<WorkerGeneration> {
    Ok(WorkerGeneration(random_bytes()?))
}

/// `N` random bytes from the OS.
pub(crate) fn random_bytes<const N: usize>() -> io::Result<[u8; N]> {
    let mut bytes = [0u8; N];
    io::Read::read_exact(&mut fs::File::open(URANDOM)?, &mut bytes)?;
    Ok(bytes)
}
