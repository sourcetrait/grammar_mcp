use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A diagnostic's location; `path` is equip-repo-relative, or null in a body.
#[derive(Debug, Clone, ser::Serialize)]
pub struct Source {
    pub path: Option<String>,
    pub position: [usize; 2],
}

/// One agent-facing diagnostic row.
#[derive(Debug, Clone, ser::Serialize)]
pub struct Diagnostic {
    pub kind: String,
    #[serde(skip)]
    pub severity: Severity,
    pub source: Option<Source>,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn error(
        kind: impl Into<String>,
        source: Option<Source>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            severity: Severity::Error,
            source,
            message: message.into(),
        }
    }

    pub(crate) fn warning(
        kind: impl Into<String>,
        source: Option<Source>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            severity: Severity::Warning,
            source,
            message: message.into(),
        }
    }

    pub(crate) fn bucket(diagnostics: Vec<Diagnostic>) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        for d in diagnostics {
            match d.severity {
                Severity::Error => errors.push(d),
                Severity::Warning => warnings.push(d),
            }
        }
        (errors, warnings)
    }
}

pub type GrammarMcpResult<T> = Result<T, GrammarMcpError>;

#[derive(Debug, Clone)]
pub enum GrammarMcpError {
    EquipNotRegistered {
        equip: String,
    },
    EquipAlreadyRegistered {
        equip: String,
    },
    EquipInvalidName {
        equip: String,
        reason: String,
    },
    EquipNameDenied {
        equip: String,
    },
    EquipInvalidSource {
        source_dir: String,
        reason: String,
    },
    EquipSourceMissing {
        path: String,
    },
    EquipSourcePathMismatch {
        equip: String,
        passed: String,
        registered: String,
    },
    EquipNoKinds {
        source_dir: String,
    },
    EquipNoRig {
        equip: String,
    },
    EquipViolations {
        diagnostics: Vec<Diagnostic>,
    },
    RigInvalidName {
        name: String,
        reason: String,
    },
    RigInvalidModulePath {
        module_path: String,
        reason: String,
    },
    FunctionNotDefined {
        rig: String,
        module_path: String,
        name: String,
    },
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
    NamepathInvalid {
        namepath: String,
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
    ThreadReturnedError {
        reason: String,
    },
    ModuleCircularImport {
        files: String,
    },
    Internal {
        phase: String,
        reason: String,
    },
}

impl GrammarMcpError {
    fn kind_str(&self) -> &'static str {
        match self {
            Self::EquipNotRegistered { .. } => "equip::not_registered",
            Self::EquipAlreadyRegistered { .. } => "equip::already_registered",
            Self::EquipInvalidName { .. } => "equip::invalid_name",
            Self::EquipNameDenied { .. } => "equip::name_denied",
            Self::EquipInvalidSource { .. } => "equip::invalid_source",
            Self::EquipSourceMissing { .. } => "equip::source_missing",
            Self::EquipSourcePathMismatch { .. } => "equip::source_path_mismatch",
            Self::EquipNoKinds { .. } => "equip::no_kinds",
            Self::EquipNoRig { .. } => "equip::no_rig",
            Self::RigInvalidName { .. } => "rig::invalid_name",
            Self::RigInvalidModulePath { .. } => "rig::invalid_module_path",
            Self::FunctionNotDefined { .. } => "function::not_defined",
            Self::DataInvalid { .. } => "data::invalid",
            Self::ResultInvalid { .. } => "result::invalid",
            Self::TimeoutInvalid { .. } => "timeout::invalid",
            Self::SchemaInvalid { .. } => "schema::invalid",
            Self::NamepathInvalid { .. } => "namepath::invalid",
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
            Self::ThreadReturnedError { .. } => "thread::returned_error",
            Self::ModuleCircularImport { .. } => "module::circular_import",
            Self::Internal { .. } => "internal",
            Self::EquipViolations { .. }
            | Self::DefinitionViolations { .. }
            | Self::InterfaceViolations { .. } => {
                unreachable!("violation variants render via bucket, not kind_str")
            }
        }
    }

    fn message(&self) -> String {
        match self {
            Self::EquipNotRegistered { equip } => {
                format!("equip `{}` is not registered", display_equip(equip))
            }
            Self::EquipAlreadyRegistered { equip } => {
                format!("equip `{}` is already registered", display_equip(equip))
            }
            Self::EquipInvalidName { equip, reason } => {
                format!("invalid equip name `{equip}`: {reason}")
            }
            Self::EquipNameDenied { equip } => {
                format!("equip name `{equip}` is reserved and cannot be used")
            }
            Self::EquipInvalidSource { source_dir, reason } => {
                format!("invalid equip source `{source_dir}`: {reason}")
            }
            Self::EquipSourceMissing { path } => {
                format!("equip source path is missing or not a directory: {path}")
            }
            Self::EquipSourcePathMismatch {
                equip,
                passed,
                registered,
            } => format!(
                "source_dir mismatch for `{}`: passed `{passed}`, registered `{registered}`",
                display_equip(equip),
            ),
            Self::EquipNoKinds { source_dir } => {
                format!("equip source `{source_dir}` holds neither a lib/ nor a rig/")
            }
            Self::EquipNoRig { equip } => format!("equip `{}` has no rig", display_equip(equip)),
            Self::RigInvalidName { name, reason } => {
                format!("invalid name `{name}`: {reason}")
            }
            Self::RigInvalidModulePath {
                module_path,
                reason,
            } => format!("invalid module path `{module_path}`: {reason}"),
            Self::FunctionNotDefined {
                rig,
                module_path,
                name,
            } => format!(
                "function `{name}` is not defined in `{}:{module_path}`",
                display_equip(rig),
            ),
            Self::DataInvalid { reason }
            | Self::ResultInvalid { reason }
            | Self::TimeoutInvalid { reason } => reason.clone(),
            Self::SchemaInvalid { reason } => format!("invalid schema: {reason}"),
            Self::NamepathInvalid { namepath, reason } => {
                format!("invalid namepath `{namepath}`: {reason}")
            }
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
            Self::ThreadReturnedError { reason } => reason.clone(),
            Self::ModuleCircularImport { files } => files.clone(),
            Self::Internal { phase, reason } => format!("internal error [{phase}]: {reason}"),
            Self::EquipViolations { .. }
            | Self::DefinitionViolations { .. }
            | Self::InterfaceViolations { .. } => {
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

/// A failure, `{nonce, error}`: the error's rows, its warnings dropped.
pub(crate) fn error_to_call_result(
    error: GrammarMcpError,
    nonce: datum::Nonce,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    let rows = match error {
        GrammarMcpError::EquipViolations { diagnostics }
        | GrammarMcpError::DefinitionViolations { diagnostics }
        | GrammarMcpError::InterfaceViolations { diagnostics } => {
            Diagnostic::bucket(diagnostics).0
        }
        single => vec![Diagnostic::error(single.kind_str(), None, single.message())],
    };
    envelope_text(nonce, "error", serde_to_nu(&rows)?)
}
