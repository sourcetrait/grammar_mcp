//! The probe's failures, and the exit statuses the qualification matrix reads.
use crate::*;

/// A probe command's result.
pub(crate) type ProbeResult<T> = Result<T, ProbeError>;

/// What ends a probe command short of a pass.
#[derive(Debug)]
pub(crate) enum ProbeError {
    /// A case that ran and failed an assertion.
    Failed {
        /// The assertion that failed, and why.
        reason: String,
    },
    /// A case this build can't run.
    Unrun {
        /// Why the case didn't run.
        reason: String,
    },
    /// A case name, a manifest, or an output path the probe refuses.
    Config {
        /// What was refused, and why.
        reason: String,
    },
    /// An output the probe couldn't write.
    Io {
        /// The output's path.
        path: PathBuf,
        /// The failure.
        reason: String,
    },
}

/// Payload-less mirror of `ProbeError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbeErrorKind {
    Failed,
    Unrun,
    Config,
    Io,
}

impl ProbeError {
    /// The failure's kind.
    pub(crate) fn kind(&self) -> ProbeErrorKind {
        match self {
            Self::Failed { .. } => ProbeErrorKind::Failed,
            Self::Unrun { .. } => ProbeErrorKind::Unrun,
            Self::Config { .. } => ProbeErrorKind::Config,
            Self::Io { .. } => ProbeErrorKind::Io,
        }
    }

    /// The exit status: 1 failed, 77 unrun, 78 refused configuration, 74 an output.
    pub(crate) fn exit_code(&self) -> u8 {
        match self.kind() {
            ProbeErrorKind::Failed => 1,
            ProbeErrorKind::Unrun => 77,
            ProbeErrorKind::Config => 78,
            ProbeErrorKind::Io => 74,
        }
    }
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed { reason } => write!(f, "failed: {reason}"),
            Self::Unrun { reason } => write!(f, "unrun: {reason}"),
            Self::Config { reason } => write!(f, "refused: {reason}"),
            Self::Io { path, reason } => write!(f, "{}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for ProbeError {}
