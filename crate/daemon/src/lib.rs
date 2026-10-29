pub(crate) mod artifact {}
pub(crate) mod bridge {}
#[cfg(feature = "service")]
pub(crate) mod cli {
    pub(crate) mod command;
}
pub(crate) mod error;
pub(crate) mod event {}
pub(crate) mod host {}
pub(crate) mod operation {}
pub(crate) mod peer {}
pub(crate) mod rpc {}
#[cfg(feature = "service")]
pub(crate) mod run;
pub(crate) mod runtime {
    pub(crate) mod start;
}
pub(crate) mod session {}
pub(crate) mod transport {}

#[cfg(feature = "service")]
pub(crate) use crate::cli::command::{GrammardCli, GrammardCmd};

pub(crate) use std::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "service")]
pub(crate) use std::process;

#[cfg(feature = "service")]
pub(crate) use clap::Parser;

pub(crate) use sourcetrait_grammar_protocol::Capabilities;

pub use crate::{
    error::{DaemonError, DaemonErrorKind, DaemonResult},
    runtime::start::{Runtime, RuntimeIdentity},
};

#[cfg(feature = "service")]
pub use crate::run::run_main;
