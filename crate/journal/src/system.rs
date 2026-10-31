//! The journal as a subsys system, and the handle its controller asks through.
use crate::*;

/// The journal's directory as the system's paths, every role its one root.
#[derive(Debug, Clone)]
pub struct JournalSysPaths {
    root: PathBuf,
}

impl JournalSysPaths {
    /// The paths of a journal kept at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl agnostic::AppPaths for JournalSysPaths {
    fn cache_dir(&self) -> PathBuf {
        self.root.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.root.clone()
    }

    fn config_secrets_dir(&self) -> PathBuf {
        self.root.clone()
    }

    fn data_dir(&self) -> PathBuf {
        self.root.clone()
    }

    fn state_dir(&self) -> PathBuf {
        self.root.clone()
    }

    fn state_secrets_dir(&self) -> PathBuf {
        self.root.clone()
    }

    fn runtime_dir(&self) -> PathBuf {
        self.root.clone()
    }
}

impl agnostic::HasAppPaths for JournalSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

/// The system's parameters: the worker it journals for, and its cap.
#[cereal::derived(Data)]
pub struct JournalSysParams {
    pub generation: WorkerGeneration,
    pub instance: Instance,
    pub cap: u64,
}

impl subsys::Params for JournalSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct JournalSysConfig;

impl subsys::Config for JournalSysConfig {}

/// The operation journal as a subsys system.
pub struct JournalSystem {
    inner: subsys::InnerSystem<Self>,
    /// Lent to each request's blocking task and given back with its answer.
    journal: Option<Journal>,
    /// The start a ticket's expiry is timed from.
    started: Instant,
}

/// The source of every id a ticket draws.
const URANDOM: &str = "/dev/urandom";

/// A ticket's operation id and engine nonce, drawn from the OS.
fn draw_ids() -> io::Result<(OperationId, datum::Nonce)> {
    let mut bytes = [0u8; 24];
    io::Read::read_exact(&mut fs::File::open(URANDOM)?, &mut bytes)?;
    let (id, nonce) = bytes.split_at(16);
    let id: [u8; 16] = id.try_into().map_err(io::Error::other)?;
    let nonce: [u8; 8] = nonce.try_into().map_err(io::Error::other)?;
    Ok((OperationId(id), datum::Nonce::new_unchecked(u64::from_be_bytes(nonce))))
}

/// One request served on the journal at `now_ms`.
fn serve(journal: &mut Journal, now_ms: u64, request: ToJournalSys) -> FromJournalSys {
    match request {
        ToJournalSys::Issue(IssueRequest { epoch }) => FromJournalSys::Issued(journal.issue(now_ms, epoch, &mut draw_ids)),
        ToJournalSys::Submit(SubmitRequest { submission, epoch, reserve }) => {
            FromJournalSys::Submitted(journal.submit(now_ms, &submission, epoch, reserve))
        }
        ToJournalSys::Advance(AdvanceRequest { operation, step }) => FromJournalSys::Moved(journal.advance(operation, step)),
        ToJournalSys::Finish(FinishRequest { operation, ending, outcome }) => {
            FromJournalSys::Moved(journal.finish(operation, ending, &outcome))
        }
        ToJournalSys::Lookup(LookupRequest { operation }) => FromJournalSys::Status(journal.lookup(operation)),
    }
}

impl subsys::System for JournalSystem {
    const CHANNEL_SIZE: usize = 256;

    type Paths = JournalSysPaths;
    type Params = JournalSysParams;
    type Config = JournalSysConfig;
    type ToSys = ToJournalSys;
    type FromSys = FromJournalSys;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> {
        &mut self.inner
    }

    async fn init(inner: subsys::InnerSystem<Self>, params: Self::Params) -> subsys::SubsysResult<Self> {
        let spec = JournalSpec {
            root: inner.paths.root.clone(),
            generation: params.generation,
            instance: params.instance,
            cap: params.cap,
        };
        let journal = tk::spawn_blocking(move || Journal::open(spec, Faults::none()))
            .await
            .map_err(|_| subsys::SubsysError::TaskJoin)?
            .map_err(|e| subsys::SubsysError::IO { source: e.into_io() })?;
        Ok(Self { inner, journal: Some(journal), started: Instant::now() })
    }

    async fn run(mut self) -> subsys::RunResult {
        let result = loop {
            let flow = match self.inner.channel.rx.recv().await {
                Some(msg) => self.handle_channel_recv(msg).await,
                None => break subsys::SUCCESS,
            };
            match flow {
                Ok(subsys::Flow::Control(subsys::Control::Stop { .. })) => break subsys::SUCCESS,
                Ok(_) => {}
                Err(e) => break Err(e),
            }
        };
        self.done(result).await
    }

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToJournalSys>) -> subsys::FlowResult<Self::Flow> {
        let (id, _nature, request) = pkt.into_tuple();
        let now_ms = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut journal = self.journal.take().ok_or(subsys::Failure)?;
        let (journal, answer) = tk::spawn_blocking(move || {
            let answer = serve(&mut journal, now_ms, request);
            (journal, answer)
        })
        .await
        .map_err(|_| subsys::Failure)?;
        self.journal = Some(journal);
        let response = subsys::Packet::response(id, answer);
        let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(response)).await;
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// The journal's running system, and the handle its controller asks through.
pub struct JournalHandle {
    control: subsys::SystemControl<JournalSystem>,
}

impl fmt::Debug for JournalHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalHandle").finish_non_exhaustive()
    }
}

impl JournalHandle {
    /// Starts the journal's system on `root`, `handler` taking what it sends up.
    pub async fn start(
        root: PathBuf,
        params: JournalSysParams,
        handler: impl subsys::Handler<JournalSystem>,
    ) -> JournalResult<Self> {
        match subsys::SystemControl::start(JournalSysPaths::new(root.clone()), JournalSysConfig, params, handler).await {
            Ok(control) => Ok(Self { control }),
            Err(subsys::SubsysError::IO { source }) => Err(JournalError::from_io(&root, &source)),
            Err(other) => Err(JournalError::Unavailable { path: root, reason: other.to_string() }),
        }
    }

    /// Sends a request and awaits its answer.
    pub async fn request<T>(&self, req: T) -> Result<T::ResponseType, String>
    where
        T: subsys::Request<JournalSystem>,
        T::ResponseType: TryFrom<FromJournalSys, Error = subsys::SubsysError>,
    {
        self.control.request(req).await.map_err(|e| format!("the operation journal's system: {e}"))
    }

    /// Stops the journal's system, its lock let go with it.
    pub async fn stop(self) -> Result<(), String> {
        self.control.stop(true).await.map_err(|e| format!("the operation journal's system: {e}"))
    }
}
