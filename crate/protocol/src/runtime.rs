use crate::*;

/// A 128-bit identity's lowercase hex.
fn hex128(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One frontend process's identity, fresh per process.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct ClientInstanceId(pub [u8; 16]);

/// A worker process's generation, fresh at each start.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct WorkerGeneration(pub [u8; 16]);

/// A controller lease's identity.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct LeaseId(pub [u8; 16]);

/// An admitted operation's durable identity.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct OperationId(pub [u8; 16]);

impl ClientInstanceId {
    /// The identity's lowercase hex.
    pub fn hex(&self) -> String {
        hex128(&self.0)
    }
}

impl WorkerGeneration {
    /// The generation's lowercase hex.
    pub fn hex(&self) -> String {
        hex128(&self.0)
    }
}

impl LeaseId {
    /// The lease's lowercase hex.
    pub fn hex(&self) -> String {
        hex128(&self.0)
    }
}

impl OperationId {
    /// The operation's lowercase hex.
    pub fn hex(&self) -> String {
        hex128(&self.0)
    }
}
