use crate::*;

pub type GrammarMcpResult<T> = Result<T, GrammarMcpError>;

#[derive(Debug, Clone)]
pub enum GrammarMcpError {
    /// What the engine refused or failed at, its kinds and messages its own.
    Engine(EngineError),
    DefinitionViolations {
        diagnostics: Vec<Diagnostic>,
    },
    InterfaceViolations {
        diagnostics: Vec<Diagnostic>,
    },
    DataInvalid {
        reason: String,
    },
    ResultInvalid {
        reason: String,
    },
    TimeoutInvalid {
        reason: String,
    },
    SchemaInvalid {
        reason: String,
    },
    PurviewInvalidId {
        id: String,
        reason: String,
    },
    RerunInvalidNonce {
        nonce: String,
        reason: String,
    },
    RerunBodyMissing {
        nonce: String,
    },
    ChannelStart {
        reason: String,
    },
    ChannelNotOpen,
    ChannelNotClaimed,
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
    ThreadDispatch {
        reason: String,
    },
    ThreadTimeout {
        timeout_ms: u64,
    },
    Internal {
        phase: String,
        reason: String,
    },
}

impl GrammarMcpError {
    fn kind_str(&self) -> &'static str {
        match self {
            Self::Engine(error) => error.kind(),
            Self::DataInvalid { .. } => "data::invalid",
            Self::ResultInvalid { .. } => "result::invalid",
            Self::TimeoutInvalid { .. } => "timeout::invalid",
            Self::SchemaInvalid { .. } => "schema::invalid",
            Self::PurviewInvalidId { .. } => "purview::invalid_id",
            Self::RerunInvalidNonce { .. } => "rerun::invalid_nonce",
            Self::RerunBodyMissing { .. } => "rerun::body_missing",
            Self::ChannelStart { .. } => "channel::start",
            Self::ChannelNotOpen => "channel::not_open",
            Self::ChannelNotClaimed => "channel::not_claimed",
            Self::ChannelPeerGone => "channel::peer_gone",
            Self::RemoteInvalidParams { .. } => "remote::invalid_params",
            Self::RemoteAlreadyOpen { .. } => "remote::already_open",
            Self::RemoteNotOpen { .. } => "remote::not_open",
            Self::RemoteOpenFailed { .. } => "remote::open_failed",
            Self::RemoteChannelUnverified => "remote::channel_unverified",
            Self::ThreadDispatch { .. } => "thread::dispatch",
            Self::ThreadTimeout { .. } => "thread::timeout",
            Self::Internal { .. } => "internal",
            Self::DefinitionViolations { .. } | Self::InterfaceViolations { .. } => {
                unreachable!("violation variants render via bucket, not kind_str")
            }
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Engine(error) => error.message(),
            Self::DataInvalid { reason }
            | Self::ResultInvalid { reason }
            | Self::TimeoutInvalid { reason } => reason.clone(),
            Self::SchemaInvalid { reason } => format!("invalid schema: {reason}"),
            Self::PurviewInvalidId { id, reason } => {
                format!("invalid purview id `{id}`: {reason}")
            }
            Self::RerunInvalidNonce { nonce, reason } => {
                format!("invalid nonce `{nonce}`: {reason}")
            }
            Self::RerunBodyMissing { nonce } => {
                format!("no cached definition for the nonce `{nonce}`; rerun takes an execute call's nonce")
            }
            Self::ChannelStart { reason } => format!("cannot start the channel hub: {reason}"),
            Self::ChannelNotOpen => {
                "the channel is not open; run `interface channel open` first".to_string()
            }
            Self::ChannelNotClaimed => "nothing has connected to the channel yet; start \
                 a Monitor at the wss endpoint `interface channel open` returned, see the \
                 channel/Open packet, then run `interface channel ready`"
                .to_string(),
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
            Self::ThreadDispatch { reason } => format!("eval dispatch failed: {reason}"),
            Self::ThreadTimeout { timeout_ms } => format!("eval timed out after {timeout_ms} ms"),
            Self::Internal { phase, reason } => format!("internal error [{phase}]: {reason}"),
            Self::DefinitionViolations { .. } | Self::InterfaceViolations { .. } => {
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
        GrammarMcpError::DefinitionViolations { diagnostics }
        | GrammarMcpError::InterfaceViolations { diagnostics } => {
            Diagnostic::bucket(diagnostics).0
        }
        single => vec![Diagnostic::error(single.kind_str(), None, single.message())],
    };
    envelope_text(nonce, "error", serde_to_nu(&rows)?)
}
