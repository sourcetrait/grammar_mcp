//! The frame ceilings of each stream role, as the wire contract sets them.
use crate::*;

/// Every stream's first frame is its hello, held to this ceiling.
pub const HELLO_FRAME_MAX: u32 = 64 * 1024;

/// The RPC frame's ceiling before negotiation.
pub const RPC_FRAME_DEFAULT: u32 = 8 * 1024 * 1024;

/// The RPC frame's ceiling no negotiation passes.
pub const RPC_FRAME_HARD: u32 = 64 * 1024 * 1024;

/// The event frame's ceiling, its default and its hard limit alike.
pub const EVENT_FRAME_MAX: u32 = 1024 * 1024;

/// A payload's decode bounds, each checked before what it bounds is allocated.
pub type DecodeBudget = bitcode::Budget;

/// A payload's decode budget before negotiation: 100,000 elements, 32 MiB of heap.
pub const DECODE_BUDGET_DEFAULT: DecodeBudget = DecodeBudget {
    elements: 100_000,
    heap: 32 * 1024 * 1024,
    depth: 128,
};

/// The decoded heap no negotiation passes.
pub const DECODE_HEAP_HARD: u64 = 128 * 1024 * 1024;

/// A stream's role, fixed by its hello for the stream's life.
#[cereal::derived(Data, Copy, Eq, Hash)]
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
