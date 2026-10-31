pub(crate) mod artifact {}
pub(crate) mod bridge {}
#[cfg(feature = "service")]
pub(crate) mod cli {
    pub(crate) mod command;
    pub(crate) mod oneshot;
}
pub(crate) mod error;
pub(crate) mod event {}
pub(crate) mod host {}
pub(crate) mod instance {
    pub(crate) mod config;
    pub(crate) mod homes;
    pub(crate) mod key;
    pub(crate) mod name;
}
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
pub(crate) use crate::cli::{
    command::{GrammardCli, GrammardCmd},
    oneshot::run_oneshot,
};

pub(crate) use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "service")]
pub(crate) use std::process;

#[cfg(feature = "service")]
pub(crate) use clap::Parser;

pub(crate) use nutype::NuTypeFile;
pub(crate) use nuval::NuVal;
pub(crate) use sha2::{Digest, Sha256};

pub(crate) use sourcetrait_grammar_protocol::Capabilities;

pub(crate) mod ser {
    pub(crate) use ::serde::Deserialize;
}

pub use crate::{
    error::{DaemonError, DaemonErrorKind, DaemonResult},
    instance::{
        config::{InstanceConfig, InstanceFile, InstancePaths, LimitLines, SupervisorLines},
        homes::InstanceHomes,
        key::InstanceKey,
        name::InstanceName,
    },
    runtime::start::{Runtime, RuntimeIdentity},
};

#[cfg(feature = "service")]
pub use crate::run::run_main;
