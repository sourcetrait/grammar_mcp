pub(crate) mod artifact {}
pub(crate) mod bridge {}
#[cfg(feature = "service")]
pub(crate) mod cli {
    pub(crate) mod command;
    pub(crate) mod oneshot;
}
pub(crate) mod controller;
pub(crate) mod error;
pub(crate) mod event {}
pub(crate) mod host {}
pub(crate) mod instance {
    pub(crate) mod config;
    pub(crate) mod homes;
    pub(crate) mod key;
    pub(crate) mod name;
}
pub(crate) mod peer {}
pub(crate) mod rpc {}
#[cfg(feature = "service")]
pub(crate) mod run;
pub(crate) mod runtime {
    pub(crate) mod scratch;
    pub(crate) mod start;
}
pub(crate) mod session {
    pub(crate) mod serve;
    pub(crate) mod sessions;
}
#[cfg(test)]
mod tests {
    mod sessions;
}
pub(crate) mod transport {
    pub(crate) mod admission;
    pub(crate) mod listener;
}

#[cfg(feature = "service")]
pub(crate) use crate::cli::{
    command::{GrammardCli, GrammardCmd},
    oneshot::run_oneshot,
};

pub(crate) use std::{
    collections::BTreeMap,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[cfg(feature = "service")]
pub(crate) use std::process;

#[cfg(feature = "service")]
pub(crate) use clap::Parser;

pub(crate) use futures_util::{SinkExt, StreamExt};
pub(crate) use nutype::NuTypeFile;
pub(crate) use nuval::NuVal;
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use tokio as tk;
pub(crate) use tokio_util as tk_util;

pub(crate) use sourcetrait_common::{datum, subsys};
pub(crate) use sourcetrait_grammar_engine as engine;
pub(crate) use sourcetrait_grammar_journal::{
    CAP_DEFAULT as JOURNAL_CAP, FromJournalSys, JournalError, JournalHandle, JournalSysParams,
    JournalSystem,
};
pub(crate) use sourcetrait_grammar_protocol::{
    ARTIFACT_LIMITS_DEFAULT, Capabilities, ClientHello, DecodeBudget, EventsAnswer, EventsFrame, FrameCodec,
    Fresh, HELLO_BUDGET, HELLO_DEADLINE, HelloReply, HelloState, Instance, LIMITS_DEFAULT, LIMITS_HARD,
    LeaseAction, LeaseBinding, LeaseEpoch, LeaseId, LeaseNotice, LeaseReply, LeaseRequest, LeaseTable,
    LeaseToken, Limits, LossReason, RpcAnswer, RpcFrame, ServerHello, StatusAnswer, StatusFrame,
    StreamClose, StreamRole, TransportError, WorkerGeneration, WorkerProfile, decode_payload, framed,
    payload,
};
pub(crate) use sourcetrait_grammar_scratch::{
    HostProbe, LOCK_FILE, LockError, LockRecord, NamespaceLock, OwnedDir, Owner, current_uid, sweep,
};

pub(crate) use crate::{
    controller::GrammardController,
    runtime::start::random_bytes,
    session::{
        serve::serve,
        sessions::{Clock, Sessions, StopRing, StreamId, ticker},
    },
    transport::{
        admission::{Admission, accept},
        listener::EngineListener,
    },
};

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
    runtime::{
        scratch::WorkerScratch,
        start::{Runtime, RuntimeIdentity},
    },
};

#[cfg(feature = "service")]
pub use crate::run::run_main;
