use crate::*;

/// What the engine's controller sends: a wire request, its own, or a host answer.
#[cereal::derived(Data)]
pub enum ToEngine {
    Wire(ToEngineSys),
    Purview(PurviewRequest),
    Host(HostAnswer),
}

/// What the engine sends its controller: the answers, or a host request.
#[cereal::derived(Data)]
pub enum FromEngine {
    Wire(FromEngineSys),
    Purview(PurviewAnswer),
    Host(HostAsk),
}

/// The engine's own requests of its namespace's purviews.
#[cereal::derived(Data)]
pub enum PurviewRequest {
    List(PurviewListRequest),
    Configure(PurviewConfigureRequest),
    Resolve(PurviewResolveRequest),
    Rig(PurviewRigRequest),
}

/// Each purview request's answer.
#[cereal::derived(Data)]
pub enum PurviewAnswer {
    Listed(Result<Vec<PurviewView>, RuntimeError>),
    Rows(Result<Vec<PurviewRow>, RuntimeError>),
    Resolved(Result<PurviewResolution, RuntimeError>),
}

/// Each configured purview, its patterns as stored.
#[cereal::derived(Data, Copy, Eq)]
pub struct PurviewListRequest;

/// An id's patterns set, an empty list deleting it; the rows after.
#[cereal::derived(Data, Eq)]
pub struct PurviewConfigureRequest {
    pub id: String,
    pub patterns: Vec<String>,
}

/// Ids resolved to what they put in view, each reference expanded.
#[cereal::derived(Data, Eq)]
pub struct PurviewResolveRequest {
    pub ids: Vec<String>,
}

/// A rig installed or uninstalled, the rows adjusted for it; the rows after.
#[cereal::derived(Data, Eq)]
pub struct PurviewRigRequest {
    pub rig: String,
    pub installed: bool,
    /// The session's view, which a rig installed joins.
    pub current: Vec<String>,
}

/// What a set of ids puts in view.
#[cereal::derived(Data)]
pub struct PurviewResolution {
    /// The patterns, in order and deduplicated, each reference expanded.
    pub patterns: Vec<String>,
    /// Each id beside the patterns it names itself.
    pub views: Vec<PurviewView>,
    /// The ids no purview answers to.
    pub unknown: Vec<String>,
}

/// What an eval asks of the process hosting it.
#[cereal::derived(Data)]
pub enum HostAsk {
    /// A body's packet for the Channel.
    ChannelSend(ChannelSendData),
    /// The stop a spam verdict made, for the error's emergency.
    Stopped(ChannelSpamErrorEmergency),
    /// A body's message for a linked peer, its files named.
    RemoteSend(RemoteSendData),
    /// The configuration in force.
    Config,
    /// A setting held for a process's lifetime.
    Pin(PinAsk),
}

/// A pin's dotted key, the process it follows, and the value it holds.
#[cereal::derived(Data)]
pub struct PinAsk {
    pub key: String,
    pub pid: u32,
    pub value: f64,
}

/// The host's answer to an ask.
#[cereal::derived(Data)]
pub enum HostAnswer {
    /// Sent, under the message's id.
    Sent(String),
    /// Refused, and why.
    Refused(String),
    /// Over the hard threshold: refused, the sender's eval to be stopped.
    Stopped(HostStop),
    /// The configuration in force, mirroring the TOML.
    Config(NuVal),
    /// The pin is held.
    Pinned,
    /// The stop is reported.
    Noted,
}

/// A spam verdict's stop, as its sender acts on it and reports it.
#[cereal::derived(Data)]
pub struct HostStop {
    pub hits: u32,
    pub window_secs: u64,
    pub rate: u32,
    /// Whether the stop begins an episode, the one its emergency reports.
    pub notify: bool,
    /// The refusal the sender answers with.
    pub reason: String,
}

impl subsys::Ask<EngineSystem> for HostAsk {
    type Answer = HostAnswer;
}

impl From<HostAsk> for FromEngine {
    fn from(v: HostAsk) -> Self {
        Self::Host(v)
    }
}

impl TryFrom<ToEngine> for HostAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: ToEngine) -> subsys::SubsysResult<Self> {
        match v {
            ToEngine::Host(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// Each purview request's way into `ToEngine`, and the answer it names.
macro_rules! purview_request {
    ($($ty:ty => $variant:ident, $answer:ty),* $(,)?) => {$(
        impl From<$ty> for ToEngine {
            fn from(v: $ty) -> Self { Self::Purview(PurviewRequest::$variant(v)) }
        }

        impl subsys::Request<EngineSystem> for $ty {
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

impl TryFrom<FromEngine> for Result<Vec<PurviewView>, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(PurviewAnswer::Listed(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for Result<Vec<PurviewRow>, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(PurviewAnswer::Rows(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for Result<PurviewResolution, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(PurviewAnswer::Resolved(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// Each wire request's way into `ToEngine`.
macro_rules! wire_request {
    ($($ty:ty => $answer:ty),* $(,)?) => {$(
        impl From<$ty> for ToEngine {
            fn from(v: $ty) -> Self { Self::Wire(v.into()) }
        }

        impl subsys::Request<EngineSystem> for $ty {
            type ResponseType = $answer;
        }
    )*};
}

wire_request! {
    ExecuteRequest => ValueAnswer,
    RerunRequest => ValueAnswer,
    CallRequest => ValueAnswer,
    ControlRequest => EngineResult<ControlAnswer>,
}

impl TryFrom<FromEngine> for ValueAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Wire(answer) => answer.try_into(),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for EngineResult<ControlAnswer> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Wire(answer) => answer.try_into(),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}
