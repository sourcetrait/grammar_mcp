use crate::*;

/// The lane a watched thread ran on.
#[cereal::derived(Data, Copy, Eq)]
pub enum ThreadLane {
    Stateless,
    Stateful,
}

impl ThreadLane {
    /// The lane's name, as a hang's key and its emergency carry it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stateless => "stateless",
            Self::Stateful => "stateful",
        }
    }
}

/// A cancelled eval whose thread had not ended when the engine was read.
#[cereal::derived(Data)]
pub struct HungThread {
    pub nonce: String,
    pub tool: String,
    pub lane: ThreadLane,
    pub started_at: u64,
    pub cancelled_at: u64,
}

/// The engine as the supervisor reads it: its watched threads, permits, and jobs.
#[cereal::derived(Data)]
pub struct EngineReading {
    pub hung: Vec<HungThread>,
    pub permits_held: u64,
    pub permits_cap: u64,
    pub jobs: u64,
}

/// What the operator's manager observed of the engine and the Channel.
#[cereal::derived(Data)]
pub struct Observation {
    /// The engine's reading, none when it didn't answer in time.
    pub engine: Option<EngineReading>,
    /// Whether the host's Channel is open, which a test host samples by.
    pub listening: bool,
}

#[cereal::derived(Data)]
pub enum ToOperatorSys {
    Observe(Observation),
}

#[cereal::derived(Data)]
pub enum FromOperatorSys {
    Emergency(Emergency),
}
