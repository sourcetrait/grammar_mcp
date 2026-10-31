//! The runtime's start and shutdown, one runtime a process.
use crate::*;

/// Whether this process has started its runtime, which it does at most once.
static STARTED: AtomicBool = AtomicBool::new(false);

/// What a worker's lock record names its holder.
const LOCK_HOLDER: &str = "grammard";

/// The source of a generation's random bytes.
const URANDOM: &str = "/dev/urandom";

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
    lock: NamespaceLock,
}

impl Runtime {
    /// Starts the process's one runtime on `config`: its lock, its scratch, its engine.
    pub fn start(config: InstanceConfig) -> DaemonResult<Self> {
        if STARTED.swap(true, Ordering::AcqRel) {
            return Err(DaemonError::AlreadyStarted);
        }
        let (lock, generation, scratch) = claim(&config).inspect_err(|_| STARTED.store(false, Ordering::Release))?;
        let mut diagnostics = scratch.diagnostics().to_vec();
        if let Err(e) = start_engine(&config, &scratch, &mut diagnostics) {
            let _ = scratch.remove();
            return Err(e);
        }
        Ok(Self {
            config,
            generation,
            scratch,
            diagnostics,
            lock,
        })
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

    /// Shuts the runtime down, its scratch removed and its lock released last.
    pub fn shutdown(self) -> DaemonResult<()> {
        let Self { scratch, lock, .. } = self;
        let removed = scratch.remove();
        drop(lock);
        removed
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
fn fresh_generation() -> std::io::Result<WorkerGeneration> {
    let mut bytes = [0u8; 16];
    std::io::Read::read_exact(&mut fs::File::open(URANDOM)?, &mut bytes)?;
    Ok(WorkerGeneration(bytes))
}
