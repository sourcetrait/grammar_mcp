use crate::*;

#[cereal::derived(Data, Copy, Eq)]
#[derive(Default)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

/// A diagnostic's location; `path` is equip-repo-relative, or null in a body.
#[cereal::derived(Data)]
pub struct Source {
    pub path: Option<String>,
    pub position: [usize; 2],
}

/// One agent-facing diagnostic row.
#[cereal::derived(Data)]
pub struct Diagnostic {
    pub kind: String,
    #[serde(skip)]
    pub severity: Severity,
    pub source: Option<Source>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(
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

    pub fn warning(
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

    pub fn bucket(diagnostics: Vec<Diagnostic>) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
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

/// What the engine failed at, and why.
#[cereal::derived(Data)]
pub enum EngineError {
    /// An eval's failure, in nushell's words.
    Eval { reason: String },
    /// A module-resolution cycle, the files it ran through.
    CircularImport { files: String },
    /// The engine's start, and why it could not.
    Init { reason: String },
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
    NamepathInvalid {
        namepath: String,
        reason: String,
    },
    DefinitionViolations {
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
    RerunInvalidNonce {
        nonce: String,
        reason: String,
    },
    RerunBodyMissing {
        nonce: String,
    },
    Dispatch {
        reason: String,
    },
    Timeout {
        timeout_ms: u64,
    },
    /// A request for a host whose engine this process doesn't serve.
    HostUnserved {
        host: String,
    },
    Internal {
        phase: String,
        reason: String,
    },
}

/// The engine's result.
pub type EngineResult<T> = Result<T, EngineError>;

impl EngineError {
    /// The kind a single-condition error reports under.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Eval { .. } => "thread::returned_error",
            Self::CircularImport { .. } => "module::circular_import",
            Self::Init { .. } | Self::Internal { .. } => "internal",
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
            Self::NamepathInvalid { .. } => "namepath::invalid",
            Self::DataInvalid { .. } => "data::invalid",
            Self::ResultInvalid { .. } => "result::invalid",
            Self::TimeoutInvalid { .. } => "timeout::invalid",
            Self::RerunInvalidNonce { .. } => "rerun::invalid_nonce",
            Self::RerunBodyMissing { .. } => "rerun::body_missing",
            Self::Dispatch { .. } => "thread::dispatch",
            Self::Timeout { .. } => "thread::timeout",
            Self::HostUnserved { .. } => "host::unserved",
            Self::EquipViolations { .. } | Self::DefinitionViolations { .. } => {
                unreachable!("a violations error renders through its rows, not a kind")
            }
        }
    }

    /// The message a single-condition error carries.
    pub fn message(&self) -> String {
        match self {
            Self::Eval { reason } | Self::Init { reason } => reason.clone(),
            Self::CircularImport { files } => files.clone(),
            Self::Internal { phase, reason } => format!("internal error [{phase}]: {reason}"),
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
            Self::NamepathInvalid { namepath, reason } => {
                format!("invalid namepath `{namepath}`: {reason}")
            }
            Self::DataInvalid { reason }
            | Self::ResultInvalid { reason }
            | Self::TimeoutInvalid { reason } => reason.clone(),
            Self::RerunInvalidNonce { nonce, reason } => {
                format!("invalid nonce `{nonce}`: {reason}")
            }
            Self::RerunBodyMissing { nonce } => {
                format!("no cached definition for the nonce `{nonce}`; rerun takes an execute call's nonce")
            }
            Self::Dispatch { reason } => format!("eval dispatch failed: {reason}"),
            Self::Timeout { timeout_ms } => format!("eval timed out after {timeout_ms} ms"),
            Self::HostUnserved { host } => format!("no engine is served on the host `{host}`"),
            Self::EquipViolations { .. } | Self::DefinitionViolations { .. } => {
                unreachable!("a violations error renders through its rows, not a message")
            }
        }
    }

    /// The error rows a failure reports: a violations error's own, else one.
    pub fn rows(self) -> Vec<Diagnostic> {
        match self {
            Self::EquipViolations { diagnostics } | Self::DefinitionViolations { diagnostics } => {
                Diagnostic::bucket(diagnostics).0
            }
            single => vec![Diagnostic::error(single.kind(), None, single.message())],
        }
    }
}

impl From<String> for EngineError {
    fn from(reason: String) -> Self {
        Self::Eval { reason }
    }
}

impl From<io::Error> for EngineError {
    fn from(e: io::Error) -> Self {
        Self::Internal {
            phase: "io".to_string(),
            reason: e.to_string(),
        }
    }
}

impl Display for EngineError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::EquipViolations { diagnostics } | Self::DefinitionViolations { diagnostics } => {
                let messages: Vec<&str> =
                    diagnostics.iter().map(|d| d.message.as_str()).collect();
                f.write_str(&messages.join("; "))
            }
            single => f.write_str(&single.message()),
        }
    }
}

impl std::error::Error for EngineError {}
