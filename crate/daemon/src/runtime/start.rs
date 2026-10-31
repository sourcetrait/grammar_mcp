//! The runtime's start and shutdown, one runtime a process.
use crate::*;

/// Whether this process has started its runtime, which it does at most once.
static STARTED: AtomicBool = AtomicBool::new(false);

/// What a worker's lock record names its holder.
const LOCK_HOLDER: &str = "grammard";

/// The source of every random id the worker draws.
const URANDOM: &str = "/dev/urandom";

/// How long a shutdown waits for the worker's tasks to end.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// The threads the worker's systems and its socket's tasks run on.
const TASK_THREADS: usize = 2;

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
    tasks: tk::runtime::Runtime,
    controller: GrammardController,
    serving: Option<Serving>,
    lock: NamespaceLock,
}

/// The engine socket's service: its listener, and its tasks' stop.
#[derive(Debug)]
struct Serving {
    listener: EngineListener,
    stop: tk::sync::watch::Sender<bool>,
}

impl Runtime {
    /// Starts the process's one runtime on `config`: its lock, its scratch, its engine.
    pub fn start(config: InstanceConfig) -> DaemonResult<Self> {
        if STARTED.swap(true, Ordering::AcqRel) {
            return Err(DaemonError::AlreadyStarted);
        }
        let given_back = |e: DaemonError| {
            STARTED.store(false, Ordering::Release);
            e
        };
        let mcp_nom = datum::Nom::generate(&datum::NonceGenerator::new()).into_pair();
        let (lock, generation, scratch) = claim(&config, mcp_nom.as_str()).map_err(given_back)?;
        let started = tasks().and_then(|tasks| {
            let controller = tasks.block_on(GrammardController::start(&config, generation))?;
            Ok((tasks, controller))
        });
        let (tasks, controller) = match started {
            Ok(started) => started,
            Err(e) => {
                let _ = scratch.remove();
                return Err(given_back(e));
            }
        };
        let mut diagnostics = scratch.diagnostics().to_vec();
        let serving = start_engine(&config, &scratch, mcp_nom.as_str(), &mut diagnostics).and_then(|()| match &config.paths.socket {
            Some(path) => serve(path, &config, generation, &tasks).map(Some),
            None => Ok(None),
        });
        let serving = match serving {
            Ok(serving) => serving,
            Err(e) => {
                tasks.block_on(controller.shutdown());
                tasks.shutdown_timeout(SHUTDOWN_GRACE);
                let _ = scratch.remove();
                return Err(e);
            }
        };
        Ok(Self {
            config,
            generation,
            scratch,
            diagnostics,
            tasks,
            controller,
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

    /// Shuts the runtime down: its socket, its systems, its scratch, its lock last.
    pub fn shutdown(self) -> DaemonResult<()> {
        let Self {
            scratch,
            tasks,
            controller,
            serving,
            lock,
            ..
        } = self;
        let listener = serving.map(|Serving { listener, stop }| {
            let _ = stop.send(true);
            listener
        });
        tasks.block_on(controller.shutdown());
        tasks.shutdown_timeout(SHUTDOWN_GRACE);
        drop(listener);
        let removed = scratch.remove();
        drop(lock);
        removed
    }
}

/// The worker's task runtime, its systems' and its socket's threads.
fn tasks() -> DaemonResult<tk::runtime::Runtime> {
    tk::runtime::Builder::new_multi_thread()
        .worker_threads(TASK_THREADS)
        .thread_name("grammard")
        .enable_all()
        .build()
        .map_err(|e| DaemonError::TasksUnavailable { reason: e.to_string() })
}

/// The engine socket bound at `path`, its streams admitted on the worker's tasks.
fn serve(
    path: &Path,
    config: &InstanceConfig,
    generation: WorkerGeneration,
    tasks: &tk::runtime::Runtime,
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
        limits: config.file.limits.map(|lines| lines.limits()).unwrap_or(LIMITS_DEFAULT),
        artifacts: ARTIFACT_LIMITS_DEFAULT,
        capabilities: Capabilities::NONE,
    };
    let bound = listener.take().ok_or_else(|| unavailable("its listener is gone".to_string()))?;
    let accepted = {
        let _entered = tasks.enter();
        tk::net::UnixListener::from_std(bound).map_err(|e| unavailable(e.to_string()))?
    };
    let started = std::time::Instant::now();
    let clock: Clock = Arc::new(move || u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
    let sessions = Arc::new(Sessions::new(generation, clock));
    let (stop, stopped) = tk::sync::watch::channel(false);
    tasks.spawn(accept(accepted, Arc::new(Admission::new(profile, sessions.clone())), stopped.clone()));
    tasks.spawn(ticker(sessions, stopped));
    Ok(Serving { listener, stop })
}


/// The lock taken and its record written, the generation drawn, the scratch made.
fn claim(
    config: &InstanceConfig,
    mcp_nom: &str,
) -> DaemonResult<(NamespaceLock, WorkerGeneration, WorkerScratch)> {
    let unavailable = |reason: String| DaemonError::LockUnavailable {
        path: config.paths.lock.clone(),
        reason,
    };
    let mut lock = NamespaceLock::acquire(&config.paths.lock)?;
    let generation = fresh_generation().map_err(|e| unavailable(format!("no generation can be drawn: {e}")))?;
    let record = LockRecord::current(LOCK_HOLDER, &config.identity.id, &config.identity.namespace, mcp_nom)
        .map_err(|e| unavailable(e.to_string()))?;
    lock.record(&record)?;
    let scratch = WorkerScratch::create(&config.homes)?;
    Ok((lock, generation, scratch))
}

/// The engine started once on the instance's homes and scratch, its substrate made.
fn start_engine(
    config: &InstanceConfig,
    scratch: &WorkerScratch,
    mcp_nom: &str,
    diagnostics: &mut Vec<String>,
) -> DaemonResult<()> {
    let failed = |reason: String| DaemonError::EngineInit { reason };
    engine::init(engine::EngineParams {
        id: config.identity.id.clone(),
        namespace: config.identity.namespace.clone(),
        work_dir: config.workdir.clone(),
        tmp_dir: scratch.tmp().to_path_buf(),
        transient_dir: scratch.transient().to_path_buf(),
        mcp_nom: mcp_nom.to_string(),
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
