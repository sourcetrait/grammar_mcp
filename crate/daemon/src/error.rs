//! The daemon's failures, and the exit codes grammard reports them by.
use crate::*;

/// A daemon operation's result.
pub type DaemonResult<T> = Result<T, DaemonError>;

/// What the daemon refuses or fails at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonError {
    /// This process already started its one runtime.
    AlreadyStarted,
    /// A command this build doesn't implement.
    Unimplemented {
        /// The command, by the name its user typed.
        command: &'static str,
    },
}

/// Payload-less mirror of `DaemonError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonErrorKind {
    AlreadyStarted,
    Unimplemented,
}

impl DaemonError {
    /// The failure's kind.
    pub fn kind(&self) -> DaemonErrorKind {
        match self {
            Self::AlreadyStarted => DaemonErrorKind::AlreadyStarted,
            Self::Unimplemented { .. } => DaemonErrorKind::Unimplemented,
        }
    }

    /// The exit code grammard reports the failure by, sysexits' meanings.
    pub fn exit_code(&self) -> u8 {
        match self.kind() {
            DaemonErrorKind::AlreadyStarted => 70,
            DaemonErrorKind::Unimplemented => 69,
        }
    }
}

impl fmt::Display for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyStarted => write!(f, "this process already started its runtime"),
            Self::Unimplemented { command } => {
                write!(f, "{command} is not implemented in this build")
            }
        }
    }
}

impl std::error::Error for DaemonError {}
