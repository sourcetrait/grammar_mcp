//! The runtime's start and shutdown, one runtime a process.
use crate::*;

/// Whether this process has started its runtime, which it does at most once.
static STARTED: AtomicBool = AtomicBool::new(false);

/// A worker's immutable identity: its effective UID, MCP id, and namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdentity {
    /// The effective Unix UID the worker runs as.
    pub uid: u32,
    /// The MCP id owning the state namespace.
    pub id: String,
    /// The state namespace within the id.
    pub namespace: String,
}

/// The persistent worker's runtime, which its host starts and shuts down.
#[derive(Debug)]
pub struct Runtime {
    identity: RuntimeIdentity,
}

impl Runtime {
    /// Starts the process's one runtime under its identity.
    pub fn start(identity: RuntimeIdentity) -> DaemonResult<Self> {
        if STARTED.swap(true, Ordering::AcqRel) {
            return Err(DaemonError::AlreadyStarted);
        }
        Ok(Self { identity })
    }

    /// The identity the runtime started under.
    pub fn identity(&self) -> &RuntimeIdentity {
        &self.identity
    }

    /// What this build of the runtime can do.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities::NONE
    }

    /// Shuts the runtime down; its process starts no other.
    pub fn shutdown(self) -> DaemonResult<()> {
        Ok(())
    }
}
