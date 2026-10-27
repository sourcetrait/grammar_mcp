use crate::*;

pub type GrammarMcpResult<T> = Result<T, GrammarMcpError>;

#[derive(Debug, Clone)]
pub enum GrammarMcpError {
    /// What the engine refused or failed at, its kinds and messages its own.
    Engine(EngineError),
    InterfaceViolations {
        diagnostics: Vec<Diagnostic>,
    },
    HostUnknown {
        host: String,
    },
    SchemaInvalid {
        reason: String,
    },
    PurviewInvalidId {
        id: String,
        reason: String,
    },
    ChannelStart {
        reason: String,
    },
    ChannelNotOpen,
    ChannelNotClaimed {
        mode: ChannelMode,
    },
    ChannelWrongMode {
        open: ChannelMode,
        asked: ChannelMode,
    },
    ChannelPeerGone,
    RemoteInvalidParams {
        reason: String,
    },
    RemoteAlreadyOpen {
        alias: String,
    },
    RemoteNotOpen {
        alias: String,
    },
    RemoteOpenFailed {
        alias: String,
        reason: String,
    },
    RemoteChannelUnverified,
    Internal {
        phase: String,
        reason: String,
    },
}

impl GrammarMcpError {
    fn kind_str(&self) -> &'static str {
        match self {
            Self::Engine(error) => error.kind(),
            Self::HostUnknown { .. } => "host::unknown",
            Self::SchemaInvalid { .. } => "schema::invalid",
            Self::PurviewInvalidId { .. } => "purview::invalid_id",
            Self::ChannelStart { .. } => "channel::start",
            Self::ChannelNotOpen => "channel::not_open",
            Self::ChannelNotClaimed { .. } => "channel::not_claimed",
            Self::ChannelWrongMode { .. } => "channel::wrong_mode",
            Self::ChannelPeerGone => "channel::peer_gone",
            Self::RemoteInvalidParams { .. } => "remote::invalid_params",
            Self::RemoteAlreadyOpen { .. } => "remote::already_open",
            Self::RemoteNotOpen { .. } => "remote::not_open",
            Self::RemoteOpenFailed { .. } => "remote::open_failed",
            Self::RemoteChannelUnverified => "remote::channel_unverified",
            Self::Internal { .. } => "internal",
            Self::InterfaceViolations { .. } => {
                unreachable!("violation variants render via bucket, not kind_str")
            }
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Engine(error) => error.message(),
            Self::HostUnknown { host } => format!(
                "unknown host `{host}`: `{LOCAL_HOST}` is this MCP's own host, \
                 and no other is configured"
            ),
            Self::SchemaInvalid { reason } => format!("invalid schema: {reason}"),
            Self::PurviewInvalidId { id, reason } => {
                format!("invalid purview id `{id}`: {reason}")
            }
            Self::ChannelStart { reason } => format!("cannot start the channel hub: {reason}"),
            Self::ChannelNotOpen => {
                "the channel is not open; run `interface channel open` first".to_string()
            }
            Self::ChannelNotClaimed {
                mode: ChannelMode::Async,
            } => "nothing has connected to the channel yet; start \
                 a Monitor at the wss endpoint `interface channel open` returned, see the \
                 channel/Open packet, then run `interface channel ready`"
                .to_string(),
            Self::ChannelNotClaimed {
                mode: ChannelMode::Sync,
            } => "nothing has taken the channel's packets yet; run `interface channel \
                 sync`, see the channel/Open packet, then run `interface channel ready`"
                .to_string(),
            Self::ChannelWrongMode { open, asked } => format!(
                "the channel is open in `{}` mode, not `{}`; close it with \
                 `interface channel close` to open it in `{}`",
                open.name(),
                asked.name(),
                asked.name(),
            ),
            Self::ChannelPeerGone => "the channel's peer connection is gone; run \
                 `interface channel close` then `interface channel open` for a fresh channel"
                .to_string(),
            Self::RemoteInvalidParams { reason } => {
                format!("invalid remote link parameters: {reason}")
            }
            Self::RemoteAlreadyOpen { alias } => format!("remote link `{alias}` is already open"),
            Self::RemoteNotOpen { alias } => format!("no open remote link `{alias}`"),
            Self::RemoteOpenFailed { alias, reason } => {
                format!("remote link `{alias}` failed to open: {reason}")
            }
            Self::RemoteChannelUnverified => "the channel is not verified; a remote link's \
                 notices ride it, so open and verify the channel before the link"
                .to_string(),
            Self::Internal { phase, reason } => format!("internal error [{phase}]: {reason}"),
            Self::InterfaceViolations { .. } => {
                unreachable!("violation variants render via bucket, not message")
            }
        }
    }
}

impl From<io::Error> for GrammarMcpError {
    fn from(e: io::Error) -> Self {
        GrammarMcpError::Internal {
            phase: "io".to_string(),
            reason: e.to_string(),
        }
    }
}

impl From<EngineError> for GrammarMcpError {
    fn from(e: EngineError) -> Self {
        GrammarMcpError::Engine(e)
    }
}

/// A failure, `{nonce, error}`: the error's rows, its warnings dropped.
pub(crate) fn error_to_call_result(
    error: impl Into<GrammarMcpError>,
    nonce: datum::Nonce,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    let rows = match error.into() {
        GrammarMcpError::Engine(error) => error.rows(),
        GrammarMcpError::InterfaceViolations { diagnostics } => Diagnostic::bucket(diagnostics).0,
        single => vec![Diagnostic::error(single.kind_str(), None, single.message())],
    };
    envelope_text(nonce, "error", serde_to_nu(&rows)?)
}
