use crate::*;

/// Where a request runs: this process's engine, or a linked host's.
#[cereal::derived(Data, Copy, Eq)]
pub enum Host {
    Local,
    Remote(datum::Nom),
}

/// An `execute` call: its definition run on its data.
#[cereal::derived(Data)]
pub struct ExecuteRequest {
    pub host: Host,
    pub nonce: datum::Nonce,
    pub definition: String,
    /// The definition's data as NUON text.
    pub data: String,
    /// The call's timeout as a Nu duration's text; the default when absent.
    pub timeout: Option<String>,
}

/// A rerun: an earlier `execute` call's definition run on fresh data.
#[cereal::derived(Data)]
pub struct RerunRequest {
    pub host: Host,
    pub nonce: datum::Nonce,
    /// The earlier call's nonce, whose cached definition runs again.
    pub source: String,
    pub data: String,
    pub timeout: Option<String>,
}

/// A call: a committed rig function run on its arguments.
#[cereal::derived(Data)]
pub struct CallRequest {
    pub host: Host,
    pub nonce: datum::Nonce,
    pub namepath: String,
    /// The function's `$args`, a record.
    pub args: NuVal,
    pub timeout_ms: Option<u64>,
}

/// A command to the engine itself.
#[cereal::derived(Data)]
pub struct ControlRequest {
    pub host: Host,
    pub command: EngineCommand,
}

/// What the engine is asked of its calls, its health, its equips, its signatures.
#[cereal::derived(Data)]
pub enum EngineCommand {
    Processes,
    Kill { nonce: String },
    Health,
    Teardown,
    EquipNew { source: String },
    EquipInstall { source: String },
    EquipCheck { source: String },
    EquipUninstall { source: String },
    EquipScaffold { namepaths: Vec<String> },
    EquipCommit { namepath: String },
    Signatures { patterns: Vec<String> },
    Help { namepath: String },
}

/// A call's answer: its nonce, and its value or the engine's refusal.
#[cereal::derived(Data)]
pub struct ValueAnswer {
    pub nonce: datum::Nonce,
    pub result: EngineResult<NuVal>,
}

/// A command's answer.
#[cereal::derived(Data)]
pub enum ControlAnswer {
    Done,
    Processes(Vec<ProcessRow>),
    Health(EngineHealth),
    Created(Vec<String>),
    Installed { equip: String, result: CommitResult },
    Checked(CheckSummary),
    /// The equip uninstalled, `None` when none was installed.
    Uninstalled { equip: Option<String> },
    Committed(CommitResult),
    Signatures(String),
    Help(InspectDoc),
}

/// One call in flight.
#[cereal::derived(Data)]
pub struct ProcessRow {
    pub nonce: String,
    pub tool: String,
    pub started_at: u64,
    pub args: NuVal,
    pub source_nonce: Option<String>,
    pub path: Option<String>,
}

/// A cancelled eval whose thread has not ended, watched for a hang.
#[cereal::derived(Data)]
pub struct HungEval {
    pub nonce: String,
    pub tool: String,
    pub lane: Lane,
    pub started_at: u64,
    pub cancelled_at: u64,
}

/// The engine's health as its supervisor samples it.
#[cereal::derived(Data)]
pub struct EngineHealth {
    pub hung: Vec<HungEval>,
    pub permits_held: u64,
    pub permits_cap: u64,
    pub jobs: u64,
}

/// An equip check's verdict: `ok` when no row is an error.
#[cereal::derived(Data)]
pub struct CheckSummary {
    pub ok: bool,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

#[cereal::derived(Data)]
pub enum ToEngineSys {
    Execute(ExecuteRequest),
    Rerun(RerunRequest),
    Call(CallRequest),
    Control(ControlRequest),
}

#[cereal::derived(Data)]
pub enum FromEngineSys {
    Value(ValueAnswer),
    Control(EngineResult<ControlAnswer>),
}

impl From<ExecuteRequest> for ToEngineSys {
    fn from(v: ExecuteRequest) -> Self { Self::Execute(v) }
}

impl From<RerunRequest> for ToEngineSys {
    fn from(v: RerunRequest) -> Self { Self::Rerun(v) }
}

impl From<CallRequest> for ToEngineSys {
    fn from(v: CallRequest) -> Self { Self::Call(v) }
}

impl From<ControlRequest> for ToEngineSys {
    fn from(v: ControlRequest) -> Self { Self::Control(v) }
}

impl subsys::Request<EngineSystem> for ExecuteRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<EngineSystem> for RerunRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<EngineSystem> for CallRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<EngineSystem> for ControlRequest {
    type ResponseType = EngineResult<ControlAnswer>;
}

impl TryFrom<FromEngineSys> for ValueAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngineSys) -> subsys::SubsysResult<Self> {
        match v {
            FromEngineSys::Value(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngineSys> for EngineResult<ControlAnswer> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngineSys) -> subsys::SubsysResult<Self> {
        match v {
            FromEngineSys::Control(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}
