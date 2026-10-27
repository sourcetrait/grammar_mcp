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
}

/// The engine's result.
pub type EngineResult<T> = Result<T, EngineError>;

impl From<String> for EngineError {
    fn from(reason: String) -> Self {
        Self::Eval { reason }
    }
}

impl Display for EngineError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Eval { reason } | Self::Init { reason } => f.write_str(reason),
            Self::CircularImport { files } => f.write_str(files),
        }
    }
}

impl std::error::Error for EngineError {}
