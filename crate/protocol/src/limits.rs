//! The frame ceilings of each stream role, as the wire contract sets them.

/// Every stream's first frame is its hello, held to this ceiling.
pub const HELLO_FRAME_MAX: u32 = 64 * 1024;

/// The RPC frame's ceiling before negotiation.
pub const RPC_FRAME_DEFAULT: u32 = 8 * 1024 * 1024;

/// The RPC frame's ceiling no negotiation passes.
pub const RPC_FRAME_HARD: u32 = 64 * 1024 * 1024;

/// The event frame's ceiling, its default and its hard limit alike.
pub const EVENT_FRAME_MAX: u32 = 1024 * 1024;

/// A stream's role, fixed by its hello for the stream's life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamRole {
    /// The controller's RPC stream.
    Rpc,
    /// The controller's event stream.
    Events,
    /// The read-only status stream.
    Status,
}

impl StreamRole {
    /// The role's frame ceiling before negotiation.
    pub const fn default_ceiling(self) -> u32 {
        match self {
            Self::Rpc | Self::Status => RPC_FRAME_DEFAULT,
            Self::Events => EVENT_FRAME_MAX,
        }
    }

    /// The role's frame ceiling no negotiation passes.
    pub const fn hard_ceiling(self) -> u32 {
        match self {
            Self::Rpc | Self::Status => RPC_FRAME_HARD,
            Self::Events => EVENT_FRAME_MAX,
        }
    }
}
