//! The journal's failures: its open's, and an admitted operation's step's.
use crate::*;

/// A journal's open's result.
pub type JournalResult<T> = Result<T, JournalError>;

/// Why a journal can't open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    /// Another process holds the journal's lock.
    Held { path: PathBuf },
    /// A directory or a file the journal can't make, open, or trust.
    Unavailable { path: PathBuf, reason: String },
}

/// Payload-less mirror of `JournalError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalErrorKind {
    Held,
    Unavailable,
}

impl JournalError {
    /// The failure's kind.
    pub fn kind(&self) -> JournalErrorKind {
        match self {
            Self::Held { .. } => JournalErrorKind::Held,
            Self::Unavailable { .. } => JournalErrorKind::Unavailable,
        }
    }

    /// The path the failure is about.
    pub fn path(&self) -> &Path {
        match self {
            Self::Held { path } | Self::Unavailable { path, .. } => path,
        }
    }

    /// The failure carried whole in an I/O error, as a system's start reports it.
    pub(crate) fn into_io(self) -> io::Error {
        io::Error::other(self)
    }

    /// A start's failure from the I/O error its system carried, under `root`.
    pub(crate) fn from_io(root: &Path, error: &io::Error) -> Self {
        match error.get_ref().and_then(|inner| inner.downcast_ref::<Self>()) {
            Some(carried) => carried.clone(),
            None => Self::Unavailable { path: root.to_path_buf(), reason: error.to_string() },
        }
    }
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Held { path } => write!(f, "another process holds the journal's lock {}", path.display()),
            Self::Unavailable { path, reason } => write!(f, "{}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for JournalError {}

/// Why an admitted operation's move wasn't recorded.
#[cereal::derived(Data, Eq)]
pub enum StepError {
    /// The journal records nothing more since a write failed, and why.
    Degraded { reason: String },
    /// The operation is unknown here, or has no such move from its state.
    Refused {
        operation: OperationId,
        state: Option<OperationState>,
    },
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Degraded { reason } => write!(f, "the journal records nothing more: {reason}"),
            Self::Refused { operation, state: Some(state) } => {
                write!(f, "operation {} has no such move from {state:?}", operation.hex())
            }
            Self::Refused { operation, state: None } => {
                write!(f, "operation {} is not admitted here", operation.hex())
            }
        }
    }
}
