//! mcp's controller: the process's systems started, routed, and stopped in order.
use crate::*;

/// How long the shutdown waits for in-flight requests to let the engine go.
#[cfg(feature = "embedded-cli")]
const ENGINE_RELEASE_GRACE: tk::TkDuration = tk::TkDuration::from_secs(1);

/// A system's status change, logged unless a shutdown is running.
pub(crate) fn log_status(
    component: &str,
    sub: subsys::FromSub,
    stopping: &AtomicBool,
) {
    if stopping.load(Ordering::SeqCst) {
        return;
    }
    if let subsys::FromSub::StatusChange(change) = sub {
        eprintln!("grammar: the {component}'s system: {:?}", change.msg().0);
    }
}

/// A system's end its control did not ask for, logged unless a shutdown is running.
pub(crate) fn log_closed(
    component: &str,
    stopping: &AtomicBool,
) {
    if !stopping.load(Ordering::SeqCst) {
        eprintln!("grammar: the {component}'s system ended unasked");
    }
}

/// A slot's lock, a poisoned one taken as it stands.
fn held<T>(slot: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|e| e.into_inner())
}

/// The operator and the observer that feeds it, the serve path's supervision.
#[cfg(feature = "embedded-cli")]
struct Supervision {
    operator: Arc<OperatorHandle>,
    observer: tk::JoinHandle<()>,
}

/// The process's systems: the face's in every build, the engine's when embedded.
pub(crate) struct McpController {
    /// The runtime config the process resolved, which every system starts on.
    pub(crate) config: Arc<Config>,
    /// The Channel's and the peer links' systems, and the open's hub.
    pub(crate) face: ChannelFace,
    /// The engine's system, none before its start and once the shutdown takes it.
    #[cfg(feature = "embedded-cli")]
    engine: std::sync::Mutex<Option<Arc<EngineHandle>>>,
    /// The operator and its observer, when the controller supervises.
    #[cfg(feature = "embedded-cli")]
    supervision: std::sync::Mutex<Option<Supervision>>,
    /// Set as the shutdown starts, so the systems' ends it asks for go unsaid.
    stopping: Arc<AtomicBool>,
    /// Held across the shutdown, so a second caller waits for the first's.
    stopped: tk::AsyncMutex<bool>,
    /// The signal sweep, which an ordinary end stands down.
    sweep: std::sync::Mutex<Option<tk::JoinHandle<()>>>,
}

impl McpController {
    /// The face's systems on `config`, the controller of a client's serving.
    pub(crate) async fn client(config: Arc<Config>) -> Result<Arc<Self>, String> {
        let face = Self::start_face(&config).await?;
        Ok(Arc::new(Self::of(config, face)))
    }

    /// The face's systems, then the engine's, and the operator's when `supervise`.
    #[cfg(feature = "embedded-cli")]
    pub(crate) async fn embedded(
        config: Arc<Config>,
        supervise: bool,
    ) -> Result<Arc<Self>, String> {
        let face = Self::start_face(&config).await?;
        let controller = Self::of(config, face);
        controller.start_engine(supervise).await?;
        Ok(Arc::new(controller))
    }

    /// The Channel's and the peer links' systems, on the process's identity.
    async fn start_face(config: &Arc<Config>) -> Result<ChannelFace, String> {
        let (nonce_gen, mcp_nom) = process_identity();
        ChannelFace::start(config.clone(), nonce_gen, mcp_nom).await
    }

    /// A controller over `face`, no other system started yet.
    fn of(
        config: Arc<Config>,
        face: ChannelFace,
    ) -> Self {
        Self {
            config,
            face,
            #[cfg(feature = "embedded-cli")]
            engine: std::sync::Mutex::new(None),
            #[cfg(feature = "embedded-cli")]
            supervision: std::sync::Mutex::new(None),
            stopping: Arc::new(AtomicBool::new(false)),
            stopped: tk::AsyncMutex::new(false),
            sweep: std::sync::Mutex::new(None),
        }
    }

    /// The engine's system, its asks routed here, then the supervision if asked.
    #[cfg(feature = "embedded-cli")]
    async fn start_engine(
        &self,
        supervise: bool,
    ) -> Result<(), String> {
        let mcp_nom = self.face.mcp_nom.as_str();
        let log = emergency_log_path(&self.config.cache_home(), mcp_nom);
        let policy = EmergencyPolicy::new(self.face.channel.clone(), mcp_nom, log);
        if supervise {
            self.face.hub.install_policy(policy.clone());
        }
        let route = HostRoute {
            channel: self.face.channel.clone(),
            peer: self.face.peer.clone(),
            config: self.config.clone(),
            stopping: self.stopping.clone(),
        };
        let engine = Arc::new(EngineHandle::start(route).await.map_err(|e| e.to_string())?);
        if supervise {
            let supervision = self.supervise(engine.clone(), policy).await?;
            *held(&self.supervision) = Some(supervision);
        }
        *held(&self.engine) = Some(engine);
        Ok(())
    }

    /// The operator on the namespace's homes, its emergencies the policy's, observed.
    #[cfg(feature = "embedded-cli")]
    async fn supervise(
        &self,
        engine: Arc<EngineHandle>,
        policy: EmergencyPolicy,
    ) -> Result<Supervision, String> {
        let paths = OperatorSysPaths::new(self.config.data_home(), self.config.cache_home());
        let params = OperatorSysParams {
            mcp_nom: self.face.mcp_nom.as_str().to_string(),
            test: self.config.test,
            supervisor: self.config.supervisor,
        };
        let watch = OperatorWatch {
            policy,
            stopping: self.stopping.clone(),
        };
        let operator = Arc::new(OperatorHandle::start(paths, params, watch).await?);
        let observer = spawn_observer(engine, operator.clone(), self.face.channel.clone());
        Ok(Supervision { operator, observer })
    }

    /// The engine's system, or the failure a stopped one answers.
    #[cfg(feature = "embedded-cli")]
    fn engine(&self) -> EngineResult<Arc<EngineHandle>> {
        held(&self.engine).clone().ok_or_else(|| EngineError::Internal {
            phase: "mcp::engine".to_string(),
            reason: "the engine's system has stopped".to_string(),
        })
    }

    /// Sends a request to the engine and awaits its answer.
    #[cfg(feature = "embedded-cli")]
    pub(crate) async fn request<T>(
        &self,
        req: T,
    ) -> EngineResult<T::ResponseType>
    where
        T: subsys::Request<EngineSystem>,
        T::ResponseType: TryFrom<FromEngine, Error = subsys::SubsysError>,
    {
        self.engine()?.request(req).await
    }

    /// Sends a command to the local engine and awaits its answer.
    #[cfg(feature = "embedded-cli")]
    pub(crate) async fn control(
        &self,
        command: EngineCommand,
    ) -> EngineResult<ControlAnswer> {
        self.engine()?.control(command).await
    }

    /// Sweep on TERM, INT, or HUP: the systems stopped, then the process's exit.
    pub(crate) fn sweep_signals(self: &Arc<Self>) {
        let controller = self.clone();
        let sweep = spawn_signal_sweep(move || async move {
            held(&controller.sweep).take();
            controller.stop().await;
        });
        *held(&self.sweep) = sweep;
    }

    /// The ordinary end: the sweep stood down, then the systems stopped in order.
    pub(crate) async fn shutdown(&self) {
        if let Some(sweep) = held(&self.sweep).take() {
            sweep.abort();
        }
        self.stop().await;
    }

    /// The Channel closed for the shutdown, then the engine's side stopped, once.
    async fn stop(&self) {
        let mut stopped = self.stopped.lock().await;
        if *stopped {
            return;
        }
        self.stopping.store(true, Ordering::SeqCst);
        self.face.closer().close_for_shutdown().await;
        #[cfg(feature = "embedded-cli")]
        self.stop_engine().await;
        *stopped = true;
    }

    /// The observer, then the engine's calls, the operator, and the engine stopped.
    #[cfg(feature = "embedded-cli")]
    async fn stop_engine(&self) {
        let supervision = held(&self.supervision).take();
        let operator = match supervision {
            Some(Supervision { operator, observer }) => {
                observer.abort();
                let _ = observer.await;
                Some(operator)
            }
            None => None,
        };
        let Some(mut engine) = held(&self.engine).take() else {
            return;
        };
        if let Err(e) = engine.control(EngineCommand::Teardown).await {
            eprintln!("grammar: the engine's teardown: {e}");
        }
        if let Some(operator) = operator
            && let Ok(operator) = Arc::try_unwrap(operator)
            && let Err(e) = operator.stop().await
        {
            eprintln!("{e}");
        }
        let deadline = Instant::now() + ENGINE_RELEASE_GRACE;
        loop {
            match Arc::try_unwrap(engine) {
                Ok(handle) => {
                    if let Err(e) = handle.stop().await {
                        eprintln!("grammar: the engine's stop: {e}");
                    }
                    return;
                }
                Err(shared) if Instant::now() < deadline => {
                    engine = shared;
                    tk::sleep(tk::TkDuration::from_millis(10)).await;
                }
                Err(_) => return,
            }
        }
    }
}
