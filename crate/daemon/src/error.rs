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
    /// An id or a namespace that isn't a name.
    NameInvalid {
        /// Which of the two it was given as.
        field: &'static str,
        name: String,
    },
    /// A home the worker needs and cannot resolve to a directory.
    HomeUnavailable {
        /// The home, by its role.
        home: &'static str,
        reason: String,
    },
    /// A path past what its use allows, a socket's sockaddr among them.
    PathTooLong { path: PathBuf, limit: usize },
    /// No instance file where the instance's key puts it.
    ConfigMissing { path: PathBuf },
    /// An instance file that does not hold this instance's settings.
    ConfigInvalid { path: PathBuf, reason: String },
    /// An instance file not to trust: another's, open to others, or not plain.
    ConfigUnsafe { path: PathBuf, reason: String },
}

/// Payload-less mirror of `DaemonError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonErrorKind {
    AlreadyStarted,
    Unimplemented,
    NameInvalid,
    HomeUnavailable,
    PathTooLong,
    ConfigMissing,
    ConfigInvalid,
    ConfigUnsafe,
}

impl DaemonErrorKind {
    /// The kind's name, as a report or a test names it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AlreadyStarted => "daemon::already_started",
            Self::Unimplemented => "daemon::unimplemented",
            Self::NameInvalid => "instance::name_invalid",
            Self::HomeUnavailable => "instance::home_unavailable",
            Self::PathTooLong => "instance::path_too_long",
            Self::ConfigMissing => "instance::config_missing",
            Self::ConfigInvalid => "instance::config_invalid",
            Self::ConfigUnsafe => "instance::config_unsafe",
        }
    }
}

impl DaemonError {
    /// The failure's kind.
    pub fn kind(&self) -> DaemonErrorKind {
        match self {
            Self::AlreadyStarted => DaemonErrorKind::AlreadyStarted,
            Self::Unimplemented { .. } => DaemonErrorKind::Unimplemented,
            Self::NameInvalid { .. } => DaemonErrorKind::NameInvalid,
            Self::HomeUnavailable { .. } => DaemonErrorKind::HomeUnavailable,
            Self::PathTooLong { .. } => DaemonErrorKind::PathTooLong,
            Self::ConfigMissing { .. } => DaemonErrorKind::ConfigMissing,
            Self::ConfigInvalid { .. } => DaemonErrorKind::ConfigInvalid,
            Self::ConfigUnsafe { .. } => DaemonErrorKind::ConfigUnsafe,
        }
    }

    /// The exit code grammard reports the failure by, sysexits' meanings.
    pub fn exit_code(&self) -> u8 {
        match self.kind() {
            DaemonErrorKind::AlreadyStarted => 70,
            DaemonErrorKind::Unimplemented => 69,
            DaemonErrorKind::NameInvalid => 64,
            DaemonErrorKind::HomeUnavailable
            | DaemonErrorKind::PathTooLong
            | DaemonErrorKind::ConfigMissing
            | DaemonErrorKind::ConfigInvalid
            | DaemonErrorKind::ConfigUnsafe => 78,
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
            Self::NameInvalid { field, name } => write!(
                f,
                "the {field} `{name}` is not a name: one to 64 of letters, digits, `.`, `_`, \
                 and `-`, a letter or a digit first",
            ),
            Self::HomeUnavailable { home, reason } => write!(f, "the {home} home: {reason}"),
            Self::PathTooLong { path, limit } => {
                write!(f, "{} passes the {limit} bytes its use allows", path.display())
            }
            Self::ConfigMissing { path } => write!(f, "no instance file at {}", path.display()),
            Self::ConfigInvalid { path, reason } => {
                write!(f, "the instance file {}: {reason}", path.display())
            }
            Self::ConfigUnsafe { path, reason } => {
                write!(f, "the instance file {} is refused: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for DaemonError {}
