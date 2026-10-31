//! mcp as a subsys system: its components started, routed, and stopped in order.
use crate::*;

/// What a component's handler heard of it, for mcp's loop.
pub(crate) enum Notice {
    Status {
        component: &'static str,
        status: String,
    },
    Closed {
        component: &'static str,
    },
}

/// A component's status changes and its end, sent to mcp's loop.
pub(crate) fn notice_status(
    notices: &tk::UnboundedSender<Notice>,
    component: &'static str,
    sub: subsys::FromSub,
) {
    if let subsys::FromSub::StatusChange(change) = sub {
        let status = format!("{:?}", change.msg().0);
        let _ = notices.send(Notice::Status { component, status });
    }
}

/// The engine's status and its end heard for mcp's loop, its host asks sent up.
struct EngineWatch {
    notices: tk::UnboundedSender<Notice>,
    /// mcp's system's way to its controller, which answers the engine's asks.
    host: subsys::Requester<McpSystem>,
}

impl subsys::Handler<EngineSystem> for EngineWatch {
    fn on_packet(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<subsys::Packet<FromEngine>>> + 'static + Send {
        std::future::ready(Some(pkt))
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<ToEngine>> + 'static + Send {
        let host = self.host.clone();
        async move {
            let FromEngine::Host(ask) = pkt.take_msg() else {
                return None;
            };
            let answer = host
                .request(ask)
                .await
                .unwrap_or_else(|e| HostAnswer::Refused(format!("mcp's controller: {e}")));
            Some(ToEngine::Host(answer))
        }
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        notice_status(&self.notices, "engine", sub);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        let _ = self.notices.send(Notice::Closed { component: "engine" });
        std::future::ready(())
    }
}

/// mcp as a subsys system: the engine and, serving, the operator, managed.
pub(crate) struct McpSystem {
    inner: subsys::InnerSystem<Self>,
    engine: Option<Arc<EngineHandle>>,
    operator: Option<Arc<OperatorHandle>>,
    observer: Option<tk::JoinHandle<()>>,
    notices: tk::UnboundedReceiver<Notice>,
    shut_down: bool,
}

/// The system's parameters: the host's identity and whether it supervises.
#[cereal::derived(Data)]
pub(crate) struct McpSysParams {
    pub mcp_nom: String,
    /// The serve path's supervision: the operator, its observer, the emergency policy.
    pub supervise: bool,
    /// Whether the host runs with `--test`, the operator's sampling gate.
    pub test: bool,
    pub supervisor: SupervisorConfig,
}

impl subsys::Params for McpSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub(crate) struct McpSysConfig;

impl subsys::Config for McpSysConfig {}

/// The namespace's homes as the system's paths.
#[derive(Debug, Clone)]
pub(crate) struct McpSysPaths {
    data_home: PathBuf,
    cache_home: PathBuf,
}

impl McpSysPaths {
    /// The namespace's secrets, its keypair's directory.
    fn secrets_dir(&self) -> PathBuf {
        self.data_home.join("keypair")
    }
}

impl agnostic::AppPaths for McpSysPaths {
    fn cache_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn config_secrets_dir(&self) -> PathBuf {
        self.secrets_dir()
    }

    fn data_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_secrets_dir(&self) -> PathBuf {
        self.secrets_dir()
    }

    fn runtime_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }
}

impl agnostic::HasAppPaths for McpSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

#[cereal::derived(Data)]
pub(crate) enum ToMcpSys {
    Engine(ToEngineSys),
    /// A request of the engine's own, for its namespace's purviews.
    Purview(PurviewRequest),
    Shutdown(ShutdownRequest),
    /// The controller's answer to an engine's host ask.
    Host(HostAnswer),
}

#[cereal::derived(Data)]
pub(crate) enum FromMcpSys {
    Engine(FromEngineSys),
    Purview(PurviewAnswer),
    ShutDown(ShutdownAnswer),
    /// An engine's host ask, sent up for the controller.
    Host(HostAsk),
    /// The operator's emergency, sent up for the controller's policy.
    Emergency(Emergency),
}

/// Each purview request's way into `ToMcpSys`, and the answer it names.
macro_rules! purview_request {
    ($($ty:ty => $variant:ident, $answer:ty),* $(,)?) => {$(
        impl From<$ty> for ToMcpSys {
            fn from(v: $ty) -> Self { Self::Purview(PurviewRequest::$variant(v)) }
        }

        impl subsys::Request<McpSystem> for $ty {
            type ResponseType = $answer;
        }
    )*};
}

purview_request! {
    PurviewListRequest => List, Result<Vec<PurviewView>, RuntimeError>,
    PurviewConfigureRequest => Configure, Result<Vec<PurviewRow>, RuntimeError>,
    PurviewResolveRequest => Resolve, Result<PurviewResolution, RuntimeError>,
    PurviewRigRequest => Rig, Result<Vec<PurviewRow>, RuntimeError>,
}

impl TryFrom<FromMcpSys> for Result<Vec<PurviewView>, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            FromMcpSys::Purview(PurviewAnswer::Listed(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromMcpSys> for Result<Vec<PurviewRow>, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            FromMcpSys::Purview(PurviewAnswer::Rows(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromMcpSys> for Result<PurviewResolution, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            FromMcpSys::Purview(PurviewAnswer::Resolved(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl subsys::Ask<McpSystem> for HostAsk {
    type Answer = HostAnswer;
}

impl From<HostAsk> for FromMcpSys {
    fn from(v: HostAsk) -> Self {
        Self::Host(v)
    }
}

impl TryFrom<ToMcpSys> for HostAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: ToMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            ToMcpSys::Host(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// The components stopped in order: the engine's calls, the operator, the engine.
#[cereal::derived(Data, Copy, Eq)]
pub(crate) struct ShutdownRequest;

/// The components' shutdown, done.
#[cereal::derived(Data, Copy, Eq)]
pub(crate) struct ShutdownAnswer;

impl From<ExecuteRequest> for ToMcpSys {
    fn from(v: ExecuteRequest) -> Self { Self::Engine(v.into()) }
}

impl From<RerunRequest> for ToMcpSys {
    fn from(v: RerunRequest) -> Self { Self::Engine(v.into()) }
}

impl From<CallRequest> for ToMcpSys {
    fn from(v: CallRequest) -> Self { Self::Engine(v.into()) }
}

impl From<ControlRequest> for ToMcpSys {
    fn from(v: ControlRequest) -> Self { Self::Engine(v.into()) }
}

impl From<ShutdownRequest> for ToMcpSys {
    fn from(v: ShutdownRequest) -> Self { Self::Shutdown(v) }
}

impl subsys::Request<McpSystem> for ExecuteRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<McpSystem> for RerunRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<McpSystem> for CallRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<McpSystem> for ControlRequest {
    type ResponseType = EngineResult<ControlAnswer>;
}

impl subsys::Request<McpSystem> for ShutdownRequest {
    type ResponseType = ShutdownAnswer;
}

impl TryFrom<FromMcpSys> for ValueAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            FromMcpSys::Engine(FromEngineSys::Value(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromMcpSys> for EngineResult<ControlAnswer> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            FromMcpSys::Engine(FromEngineSys::Control(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromMcpSys> for ShutdownAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromMcpSys) -> subsys::SubsysResult<Self> {
        match v {
            FromMcpSys::ShutDown(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// The engine's answer to `msg`, or its refusal as the answer when it couldn't.
async fn forward(
    engine: Option<Arc<EngineHandle>>,
    msg: ToEngineSys,
) -> FromEngineSys {
    let gone = || EngineError::Internal {
        phase: "mcp::engine".to_string(),
        reason: "the engine's system has stopped".to_string(),
    };
    match msg {
        ToEngineSys::Execute(req) => {
            let nonce = req.nonce;
            let answer = match engine {
                Some(engine) => engine.request(req).await,
                None => Err(gone()),
            };
            FromEngineSys::Value(value_answer(nonce, answer))
        }
        ToEngineSys::Rerun(req) => {
            let nonce = req.nonce;
            let answer = match engine {
                Some(engine) => engine.request(req).await,
                None => Err(gone()),
            };
            FromEngineSys::Value(value_answer(nonce, answer))
        }
        ToEngineSys::Call(req) => {
            let nonce = req.nonce;
            let answer = match engine {
                Some(engine) => engine.request(req).await,
                None => Err(gone()),
            };
            FromEngineSys::Value(value_answer(nonce, answer))
        }
        ToEngineSys::Control(req) => {
            let answer = match engine {
                Some(engine) => engine.request(req).await.and_then(|answer| answer),
                None => Err(gone()),
            };
            FromEngineSys::Control(answer)
        }
    }
}

/// The engine's answer to a purview request, or its failure in the request's kind.
async fn forward_purview(
    engine: Option<Arc<EngineHandle>>,
    req: PurviewRequest,
) -> PurviewAnswer {
    let Some(engine) = engine else {
        return purview_refused(&req, "the engine's system has stopped".to_string());
    };
    let shape = req.clone();
    let answer = match req {
        PurviewRequest::List(req) => engine.request(req).await.map(PurviewAnswer::Listed),
        PurviewRequest::Configure(req) => engine.request(req).await.map(PurviewAnswer::Rows),
        PurviewRequest::Resolve(req) => engine.request(req).await.map(PurviewAnswer::Resolved),
        PurviewRequest::Rig(req) => engine.request(req).await.map(PurviewAnswer::Rows),
    };
    answer.unwrap_or_else(|e| purview_refused(&shape, e.to_string()))
}

/// A purview request's failure, answered in its request's own kind.
fn purview_refused(
    req: &PurviewRequest,
    reason: String,
) -> PurviewAnswer {
    let error = RuntimeError::Internal {
        phase: "mcp::engine".to_string(),
        reason,
    };
    match req {
        PurviewRequest::List(_) => PurviewAnswer::Listed(Err(error)),
        PurviewRequest::Resolve(_) => PurviewAnswer::Resolved(Err(error)),
        PurviewRequest::Configure(_) | PurviewRequest::Rig(_) => PurviewAnswer::Rows(Err(error)),
    }
}

/// A value's answer, the request's failure answered under its nonce.
fn value_answer(
    nonce: datum::Nonce,
    answer: EngineResult<ValueAnswer>,
) -> ValueAnswer {
    answer.unwrap_or_else(|error| ValueAnswer {
        nonce,
        result: Err(error),
    })
}

/// How long the shutdown waits for in-flight requests to let the engine go.
const ENGINE_RELEASE_GRACE: tk::TkDuration = tk::TkDuration::from_secs(1);

impl McpSystem {
    /// The components stopped in order: the engine's calls, the operator, the engine.
    async fn shut_down(&mut self) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        if let Some(observer) = self.observer.take() {
            observer.abort();
            let _ = observer.await;
        }
        if let Some(engine) = &self.engine
            && let Err(e) = engine.control(EngineCommand::Teardown).await
        {
            eprintln!("grammar: the engine's teardown: {e}");
        }
        if let Some(operator) = self.operator.take()
            && let Ok(operator) = Arc::try_unwrap(operator)
            && let Err(e) = operator.stop().await
        {
            eprintln!("{e}");
        }
        let Some(mut engine) = self.engine.take() else {
            return;
        };
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

    /// A component's status or end, logged; an end mcp asked for is none.
    fn on_notice(
        &self,
        notice: Notice,
    ) {
        if self.shut_down {
            return;
        }
        match notice {
            Notice::Status { component, status } => {
                eprintln!("grammar: the {component}'s system: {status}");
            }
            Notice::Closed { component } => {
                eprintln!("grammar: the {component}'s system ended unasked");
            }
        }
    }
}

impl subsys::System for McpSystem {
    const CHANNEL_SIZE: usize = 100;

    type Paths = McpSysPaths;
    type Params = McpSysParams;
    type Config = McpSysConfig;
    type ToSys = ToMcpSys;
    type FromSys = FromMcpSys;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> { &self.inner }
    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> { &mut self.inner }

    async fn init(inner: subsys::InnerSystem<Self>, params: Self::Params) -> subsys::SubsysResult<Self> {
        let (notice_tx, notices) = tk::unbounded_channel();
        let engine = EngineHandle::start(EngineWatch {
            notices: notice_tx.clone(),
            host: inner.requester(),
        })
        .await
        .map_err(|e| {
            eprintln!("{e}");
            subsys::SubsysError::Fatal
        })?;
        let engine = Arc::new(engine);
        let (operator, observer) = if params.supervise {
            let paths = OperatorSysPaths::new(
                inner.paths.data_home.clone(),
                inner.paths.cache_home.clone(),
            );
            let operator_params = OperatorSysParams {
                mcp_nom: params.mcp_nom.clone(),
                test: params.test,
                supervisor: params.supervisor,
            };
            let watch = OperatorWatch {
                up: inner.channel.tx.clone(),
                notices: notice_tx,
            };
            let operator = OperatorHandle::start(paths, operator_params, watch)
                .await
                .map_err(|e| {
                    eprintln!("{e}");
                    subsys::SubsysError::Fatal
                })?;
            let operator = Arc::new(operator);
            let observer = spawn_observer(engine.clone(), operator.clone());
            (Some(operator), Some(observer))
        } else {
            (None, None)
        };
        Ok(Self {
            inner,
            engine: Some(engine),
            operator,
            observer,
            notices,
            shut_down: false,
        })
    }

    async fn run(mut self) -> subsys::RunResult {
        let result = loop {
            let flow = tk::select! {
                msg = self.inner.channel.rx.recv() => match msg {
                    Some(msg) => self.handle_channel_recv(msg).await,
                    None => break subsys::SUCCESS,
                },
                Some(notice) = self.notices.recv() => {
                    self.on_notice(notice);
                    Ok(subsys::Flow::Continue)
                }
            };
            match flow {
                Ok(subsys::Flow::Control(subsys::Control::Stop { .. })) => break subsys::SUCCESS,
                Ok(_) => {}
                Err(e) => break Err(e),
            }
        };
        self.done(result).await
    }

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToMcpSys>) -> subsys::FlowResult<Self::Flow> {
        let (id, _nature, msg) = pkt.into_tuple();
        match msg {
            ToMcpSys::Engine(msg) => {
                let engine = self.engine.clone();
                let tx = self.inner.channel.tx.clone();
                tk::spawn(async move {
                    let answer = FromMcpSys::Engine(forward(engine, msg).await);
                    let response = subsys::Packet::response(id, answer);
                    let _ = tx.send(subsys::MsgFromSys::Packet(response)).await;
                });
            }
            ToMcpSys::Purview(req) => {
                let engine = self.engine.clone();
                let tx = self.inner.channel.tx.clone();
                tk::spawn(async move {
                    let answer = FromMcpSys::Purview(forward_purview(engine, req).await);
                    let response = subsys::Packet::response(id, answer);
                    let _ = tx.send(subsys::MsgFromSys::Packet(response)).await;
                });
            }
            ToMcpSys::Shutdown(ShutdownRequest) => {
                self.shut_down().await;
                let response = subsys::Packet::response(id, FromMcpSys::ShutDown(ShutdownAnswer));
                let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(response)).await;
            }
            ToMcpSys::Host(_) => {}
        }
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        self.shut_down().await;
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// mcp's running system, and the handle its tools request through.
pub(crate) struct McpHandle {
    control: subsys::SystemControl<McpSystem>,
}

impl McpHandle {
    /// Sends a request and awaits its answer.
    pub(crate) async fn request<T>(&self, req: T) -> EngineResult<T::ResponseType>
    where
        T: subsys::Request<McpSystem>,
        T::ResponseType: TryFrom<FromMcpSys, Error = subsys::SubsysError>,
    {
        self.control.request(req).await.map_err(|e| EngineError::Internal {
            phase: "mcp::request".to_string(),
            reason: e.to_string(),
        })
    }

    /// Sends a command to the local engine and awaits its answer.
    pub(crate) async fn control(&self, command: EngineCommand) -> EngineResult<ControlAnswer> {
        self.request(ControlRequest {
            host: Host::Local,
            command,
        })
        .await?
    }

    /// Stops the components in order, the system serving on until the process ends.
    pub(crate) async fn shutdown(&self) {
        if let Err(e) = self.request(ShutdownRequest).await {
            eprintln!("grammar: the shutdown: {e}");
        }
    }
}

/// Start mcp's system, with the operator if `supervise`, `route` its controller.
pub(crate) async fn start_system(
    mcp_nom: &datum::NomPair,
    supervise: bool,
    route: HostRoute,
) -> Result<McpHandle, String> {
    let cfg = config();
    let paths = McpSysPaths {
        data_home: data_base_dir(),
        cache_home: cache_base_dir(),
    };
    let params = McpSysParams {
        mcp_nom: mcp_nom.as_str().to_string(),
        supervise,
        test: cfg.test,
        supervisor: cfg.supervisor,
    };
    if supervise {
        route.supervise();
    }
    let control = subsys::SystemControl::start(paths, McpSysConfig, params, route)
        .await
        .map_err(|e| format!("grammar: mcp's system: {e}"))?;
    Ok(McpHandle { control })
}
