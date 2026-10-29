use crate::*;

/// The model a link's packet rides under once both its connections pair.
pub const MODEL_CONNECTED: &str = "remote/channel/Connected";

/// The model a link's packet rides under when it ends or never pairs.
pub const MODEL_DISCONNECTED: &str = "remote/channel/Disconnected";

/// The model of a send's report once its message is written to the link.
pub const MODEL_SENT: &str = "remote/channel/Sent";

/// The model of a send's report when it failed before or during its write.
pub const MODEL_UNSENT: &str = "remote/channel/Unsent";

/// A remote alias's state: closed, listening for its peer, or open.
#[cereal::derived(Data, Copy, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteLinkState {
    Closed,
    Listening,
    Open,
}

/// One remote in `interface remote channel list` and `get`: its entry and state.
#[cereal::derived(Data)]
pub struct RemoteChannelRow {
    pub alias: String,
    /// The `ip:port` a connect dials by default, as the file writes it.
    pub address: Option<String>,
    /// The `ip` or `ip:port` a listen binds by default, as the file writes it.
    pub listen: Option<String>,
    /// The peer's certificate, as the file writes it.
    pub cert: Option<String>,
    pub state: RemoteLinkState,
    /// The address bound or dialed while the remote is live.
    pub endpoint: Option<String>,
    /// The peer's nom while a link is open.
    pub remote_mcp_nom: Option<String>,
}

/// `interface remote channel listen`'s answer: the address it bound.
#[cereal::derived(Data)]
pub struct RemoteListenResult {
    /// The address bound, its port the kernel's when random.
    pub endpoint: String,
}

/// Why a link or a send failed: its kind and its message.
#[cereal::derived(Data)]
pub struct RemoteFailure {
    pub kind: String,
    pub message: String,
}

/// A link's lifecycle and a send's delivery, as their packets report them.
#[cereal::derived(Data)]
pub enum RemoteChannelEvent {
    Connected {
        remote: String,
        mcp_nom: String,
    },
    Disconnected {
        remote: String,
        mcp_nom: Option<String>,
        error: Option<RemoteFailure>,
    },
    Sent {
        id: String,
        mcp_nom: String,
    },
    Unsent {
        id: String,
        mcp_nom: String,
        error: RemoteFailure,
    },
}

impl RemoteChannelEvent {
    /// The model its packet rides under.
    pub fn model(&self) -> &'static str {
        match self {
            Self::Connected { .. } => MODEL_CONNECTED,
            Self::Disconnected { .. } => MODEL_DISCONNECTED,
            Self::Sent { .. } => MODEL_SENT,
            Self::Unsent { .. } => MODEL_UNSENT,
        }
    }
}
