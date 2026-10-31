use crate::*;

/// What the engine's controller sends it: the wire's requests, or a host's answer.
#[cereal::derived(Data)]
pub enum ToEngine {
    Wire(ToEngineSys),
    Host(HostAnswer),
}

/// What the engine sends its controller: the wire's answers, or a host request.
#[cereal::derived(Data)]
pub enum FromEngine {
    Wire(FromEngineSys),
    Host(HostAsk),
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
            ToEngine::Wire(_) => Err(subsys::SubsysError::ResponseType),
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
            FromEngine::Host(_) => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for EngineResult<ControlAnswer> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Wire(answer) => answer.try_into(),
            FromEngine::Host(_) => Err(subsys::SubsysError::ResponseType),
        }
    }
}
