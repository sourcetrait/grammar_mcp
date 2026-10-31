//! A stream's opening exchange: both hellos and the validation both ends share.
use crate::*;

/// The protocol version a hello names.
#[cereal::derived(Data, Copy, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// The version this build speaks, v1.0.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion { major: 1, minor: 0 };

/// How long a stream may go without its hello once it opens.
pub const HELLO_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// How long an SSH session may take to authenticate.
pub const AUTH_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// The bounds a hello or its reply decodes within, sized to the hello's frame.
pub const HELLO_BUDGET: DecodeBudget = DecodeBudget {
    elements: 64,
    heap: HELLO_FRAME_MAX as u64,
    depth: 8,
};

/// The most features a hello may require.
pub const REQUIRED_FEATURES_MAX: usize = 16;

/// A worker instance's name: its id and its namespace.
#[cereal::derived(Data, Eq, Hash)]
pub struct Instance {
    pub id: String,
    pub namespace: String,
}

/// Whether `name` is a valid id or namespace.
fn valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl Instance {
    /// An instance of `id` and `namespace`, each refused unless valid, never sanitized.
    pub fn new(id: &str, namespace: &str) -> Result<Self, TransportError> {
        let instance = Self {
            id: id.to_string(),
            namespace: namespace.to_string(),
        };
        instance.check()?;
        Ok(instance)
    }

    /// The instance refused unless its id and namespace are both valid.
    pub fn check(&self) -> Result<(), TransportError> {
        for (which, name) in [("id", &self.id), ("namespace", &self.namespace)] {
            if !valid_name(name) {
                return Err(TransportError::InvalidFrame {
                    reason: format!("the {which} `{name}` is not a valid instance name"),
                });
            }
        }
        Ok(())
    }

    /// The instance's key: the SHA-256 of each name's length and bytes in turn.
    pub fn key(&self) -> InstanceKey {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        for name in [&self.id, &self.namespace] {
            hash.update((name.len() as u32).to_be_bytes());
            hash.update(name.as_bytes());
        }
        InstanceKey(hash.finalize().into())
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.id, self.namespace)
    }
}

/// An instance's key, its paths' and its sockets' name.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct InstanceKey(pub [u8; 32]);

impl InstanceKey {
    /// The key's lowercase hex.
    pub fn hex(&self) -> String {
        hex(&self.0)
    }
}

/// Bytes in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bounds a hello negotiates, each the least of both ends'.
#[cereal::derived(Data, Copy, Eq)]
pub struct Limits {
    /// The RPC and Status streams' frame ceiling.
    pub rpc_frame: u32,
    /// The heap one frame's payload decodes within.
    pub decode_heap: u64,
    /// The ordinary RPCs one lease may have in flight.
    pub concurrent_rpcs: u32,
    /// The slots reserved for control and status beside them.
    pub control_slots: u32,
    /// The event backlog's records.
    pub event_records: u32,
    /// The event backlog's bytes.
    pub event_bytes: u64,
    /// The transport streams the worker accepts at once, every role's.
    pub streams: u32,
}

/// The bounds before negotiation.
pub const LIMITS_DEFAULT: Limits = Limits {
    rpc_frame: RPC_FRAME_DEFAULT,
    decode_heap: DECODE_BUDGET_DEFAULT.heap,
    concurrent_rpcs: 32,
    control_slots: 4,
    event_records: 4096,
    event_bytes: 16 * 1024 * 1024,
    streams: 16,
};

/// The bounds no negotiation passes.
pub const LIMITS_HARD: Limits = Limits {
    rpc_frame: RPC_FRAME_HARD,
    decode_heap: DECODE_HEAP_HARD,
    concurrent_rpcs: 128,
    control_slots: 8,
    event_records: 65536,
    event_bytes: 256 * 1024 * 1024,
    streams: 64,
};

impl Limits {
    /// Each bound by its name, widened.
    fn bounds(&self) -> [(&'static str, u64); 7] {
        [
            ("rpc_frame", u64::from(self.rpc_frame)),
            ("decode_heap", self.decode_heap),
            ("concurrent_rpcs", u64::from(self.concurrent_rpcs)),
            ("control_slots", u64::from(self.control_slots)),
            ("event_records", u64::from(self.event_records)),
            ("event_bytes", self.event_bytes),
            ("streams", u64::from(self.streams)),
        ]
    }

    /// Each bound the least of this one's and `other`'s.
    pub fn least(self, other: Limits) -> Limits {
        Limits {
            rpc_frame: self.rpc_frame.min(other.rpc_frame),
            decode_heap: self.decode_heap.min(other.decode_heap),
            concurrent_rpcs: self.concurrent_rpcs.min(other.concurrent_rpcs),
            control_slots: self.control_slots.min(other.control_slots),
            event_records: self.event_records.min(other.event_records),
            event_bytes: self.event_bytes.min(other.event_bytes),
            streams: self.streams.min(other.streams),
        }
    }

    /// The first bound at zero, by its name.
    pub fn zero(&self) -> Option<&'static str> {
        self.bounds().into_iter().find(|(_, value)| *value == 0).map(|(name, _)| name)
    }

    /// The first bound past `other`'s: its name, this value, and `other`'s.
    pub fn past(&self, other: &Limits) -> Option<(&'static str, u64, u64)> {
        self.bounds()
            .into_iter()
            .zip(other.bounds())
            .find(|((_, mine), (_, theirs))| mine > theirs)
            .map(|((name, mine), (_, theirs))| (name, mine, theirs))
    }

    /// The frame ceiling a stream of `role` takes under these bounds.
    pub fn frame_ceiling(&self, role: StreamRole) -> u32 {
        match role {
            StreamRole::Rpc | StreamRole::Status => self.rpc_frame,
            StreamRole::Events => EVENT_FRAME_MAX,
        }
    }

    /// The budget a frame after the hello decodes within.
    pub fn decode_budget(&self) -> DecodeBudget {
        DecodeBudget {
            heap: self.decode_heap,
            ..DECODE_BUDGET_DEFAULT
        }
    }
}

/// A stream's first frame from its client.
#[cereal::derived(Data, Eq)]
pub struct ClientHello {
    pub version: ProtocolVersion,
    pub schema: [u8; 32],
    pub codec: String,
    /// The worker instance the client means to reach.
    pub expected: Instance,
    /// The stream's role, fixed for its life.
    pub role: StreamRole,
    /// What the stream asks of the namespace's lease, as its role allows.
    pub lease: LeaseIntent,
    /// The client process's identity, fresh per process.
    pub client: ClientInstanceId,
    /// The bounds the client asks for.
    pub requested: Limits,
    /// The capabilities the client needs, by name, refused unless all are on.
    pub required: Vec<String>,
}

/// A worker's identity as its reply names it.
#[cereal::derived(Data, Eq)]
pub struct WorkerIdentity {
    /// The worker's effective Unix UID.
    pub uid: u32,
    pub instance: Instance,
    pub key: InstanceKey,
    /// The worker process's generation, fresh at each start.
    pub generation: WorkerGeneration,
}

/// The worker's acceptance of a hello: who it is, and what the stream may do.
#[cereal::derived(Data, Eq)]
pub struct ServerHello {
    pub version: ProtocolVersion,
    pub schema: [u8; 32],
    pub codec: String,
    pub worker: WorkerIdentity,
    pub role: StreamRole,
    /// The lease the stream is bound to.
    pub lease: LeaseBinding,
    /// The negotiated bounds.
    pub effective: Limits,
    /// The worker's artifact store's bounds, its own to set within the contract.
    pub artifacts: ArtifactLimits,
    pub capabilities: Capabilities,
}

/// A stream's first frame from its worker: the hello accepted, or refused.
#[cereal::derived(Data)]
pub enum HelloReply {
    Accepted(Box<ServerHello>),
    Refused(TransportError),
}

/// The version, schema, and codec a peer named, held to this build's.
fn agree(version: ProtocolVersion, schema: &[u8; 32], codec: &str) -> Result<(), TransportError> {
    if version.major != PROTOCOL_VERSION.major {
        return Err(TransportError::VersionMismatch {
            expected: PROTOCOL_VERSION.to_string(),
            found: version.to_string(),
        });
    }
    if *schema != SCHEMA_ID {
        return Err(TransportError::SchemaMismatch {
            expected: hex(&SCHEMA_ID),
            found: hex(schema),
        });
    }
    check_codec(codec).map_err(|e| e.transport())
}

/// Each required feature a capability `capabilities` has on.
fn provide(required: &[String], capabilities: Capabilities) -> Result<(), TransportError> {
    if required.len() > REQUIRED_FEATURES_MAX {
        return Err(TransportError::InvalidFrame {
            reason: format!("a hello requires {} features, past {REQUIRED_FEATURES_MAX}", required.len()),
        });
    }
    match required.iter().find(|name| capabilities.named(name) != Some(true)) {
        Some(feature) => Err(TransportError::UnsupportedFeature { feature: feature.clone() }),
        None => Ok(()),
    }
}

/// What a worker answers a hello from: its identity, bounds, and capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerProfile {
    pub uid: u32,
    pub instance: Instance,
    pub generation: WorkerGeneration,
    /// The bounds the worker allows, held to `LIMITS_HARD` as it negotiates.
    pub limits: Limits,
    /// The artifact store's bounds, advertised as they are.
    pub artifacts: ArtifactLimits,
    pub capabilities: Capabilities,
}

/// The worker's checks of `hello`, before any lease binds: its negotiated limits.
pub fn check_hello(hello: &ClientHello, worker: &WorkerProfile) -> Result<Limits, TransportError> {
    agree(hello.version, &hello.schema, &hello.codec)?;
    hello.expected.check()?;
    if hello.expected != worker.instance {
        return Err(TransportError::WrongWorker {
            expected: hello.expected.to_string(),
            found: worker.instance.to_string(),
        });
    }
    if !hello.lease.fits(hello.role) {
        return Err(TransportError::InvalidFrame {
            reason: format!("a {:?} stream can't {}", hello.role, hello.lease.name()),
        });
    }
    provide(&hello.required, worker.capabilities)?;
    if let Some(name) = hello.requested.zero() {
        return Err(TransportError::InvalidFrame {
            reason: format!("the hello requests a {name} of zero"),
        });
    }
    Ok(hello.requested.least(worker.limits).least(LIMITS_HARD))
}

/// The worker's reply to a checked hello, its lease bound.
fn accepted(hello: &ClientHello, worker: &WorkerProfile, effective: Limits, lease: LeaseBinding) -> ServerHello {
    ServerHello {
        version: PROTOCOL_VERSION,
        schema: SCHEMA_ID,
        codec: CODEC_ID.to_string(),
        worker: WorkerIdentity {
            uid: worker.uid,
            instance: worker.instance.clone(),
            key: worker.instance.key(),
            generation: worker.generation,
        },
        role: hello.role,
        lease,
        effective,
        artifacts: worker.artifacts,
        capabilities: worker.capabilities,
    }
}

/// What a client expects of the worker a stream reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    /// The worker's effective UID, from the endpoint's profile.
    pub uid: u32,
    pub instance: Instance,
    pub role: StreamRole,
    /// What the stream asks of the lease.
    pub lease: LeaseIntent,
    pub requested: Limits,
    pub required: Vec<String>,
    /// The generation the client last reached this worker at, if any.
    pub last: Option<WorkerGeneration>,
}

impl Expectation {
    /// The hello this expectation opens a stream with.
    pub fn hello(&self, client: ClientInstanceId) -> ClientHello {
        ClientHello {
            version: PROTOCOL_VERSION,
            schema: SCHEMA_ID,
            codec: CODEC_ID.to_string(),
            expected: self.instance.clone(),
            role: self.role,
            lease: self.lease.clone(),
            client,
            requested: self.requested,
            required: self.required.clone(),
        }
    }
}

/// A stream's accepted handshake, as its client holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub worker: WorkerIdentity,
    pub role: StreamRole,
    pub lease: LeaseBinding,
    pub limits: Limits,
    /// The worker's artifact store's bounds.
    pub artifacts: ArtifactLimits,
    pub capabilities: Capabilities,
    /// Whether the worker's generation is new since the expectation's last.
    pub new_generation: bool,
}

/// The client's check of the worker's reply against what it expects.
pub fn check_reply(reply: HelloReply, expect: &Expectation) -> Result<Session, TransportError> {
    let hello = match reply {
        HelloReply::Accepted(hello) => *hello,
        HelloReply::Refused(refusal) => return Err(refusal),
    };
    agree(hello.version, &hello.schema, &hello.codec)?;
    hello.worker.instance.check()?;
    if hello.worker.uid != expect.uid || hello.worker.instance != expect.instance {
        return Err(TransportError::WrongWorker {
            expected: format!("uid {} {}", expect.uid, expect.instance),
            found: format!("uid {} {}", hello.worker.uid, hello.worker.instance),
        });
    }
    if hello.worker.key != hello.worker.instance.key() {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker's key {} is not its instance's", hello.worker.key.hex()),
        });
    }
    if hello.role != expect.role {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker answered a {:?} stream as {:?}", expect.role, hello.role),
        });
    }
    if let Some((name, effective, requested)) = hello.effective.past(&expect.requested) {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker raised {name} to {effective} past the {requested} requested"),
        });
    }
    if let Some((name, effective, hard)) = hello.effective.past(&LIMITS_HARD) {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker granted {name} {effective} past the hard ceiling {hard}"),
        });
    }
    if let LeaseIntent::Resume { generation, .. } | LeaseIntent::Attach { generation, .. } = &expect.lease
        && *generation != hello.worker.generation
    {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker answered a {} from another generation", expect.lease.name()),
        });
    }
    if let Some(name) = hello.effective.zero() {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker set {name} to zero"),
        });
    }
    if let Some(bound) = hello.artifacts.out_of_contract() {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker advertised {bound}"),
        });
    }
    provide(&expect.required, hello.capabilities)?;
    if !bound_as_asked(&expect.lease, &hello.lease) {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker answered a {} with another binding", expect.lease.name()),
        });
    }
    Ok(Session {
        new_generation: expect.last.is_some_and(|last| last != hello.worker.generation),
        worker: hello.worker,
        role: hello.role,
        lease: hello.lease,
        limits: hello.effective,
        artifacts: hello.artifacts,
        capabilities: hello.capabilities,
    })
}

/// Whether `binding` answers `intent`.
fn bound_as_asked(intent: &LeaseIntent, binding: &LeaseBinding) -> bool {
    match (intent, binding) {
        (LeaseIntent::Claim | LeaseIntent::Takeover, LeaseBinding::Granted(_)) => true,
        (LeaseIntent::Resume { lease, token, epoch, .. }, LeaseBinding::Granted(grant)) => {
            grant.lease == *lease && grant.token.matches(token) && grant.epoch == *epoch
        }
        (LeaseIntent::Attach { lease, epoch, .. }, LeaseBinding::Attached { lease: bound, epoch: at }) => {
            bound == lease && at == epoch
        }
        (LeaseIntent::Observe, LeaseBinding::Unbound) => true,
        _ => false,
    }
}

/// One stream's handshake on the worker: awaiting, ready, or closed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HelloState {
    #[default]
    Awaiting,
    Ready(Box<ServerHello>),
    Closed,
}

impl HelloState {
    /// The reply to `hello` at `now_ms`, its lease bound, and the lease's actions.
    pub fn on_hello(
        &mut self,
        hello: &ClientHello,
        worker: &WorkerProfile,
        leases: &mut LeaseTable,
        now_ms: u64,
        fresh: Fresh,
    ) -> (HelloReply, Vec<LeaseAction>) {
        let (outcome, actions) = match self {
            Self::Awaiting => match check_hello(hello, worker) {
                Ok(effective) => {
                    let change = leases.bind(now_ms, hello.role, &hello.lease, fresh);
                    (change.outcome.map(|lease| accepted(hello, worker, effective, lease)), change.actions)
                }
                Err(refusal) => (Err(refusal), Vec::new()),
            },
            Self::Ready(_) => (
                Err(TransportError::InvalidFrame { reason: "a second hello on the stream".to_string() }),
                Vec::new(),
            ),
            Self::Closed => (
                Err(TransportError::InvalidFrame { reason: "a hello on a closed stream".to_string() }),
                Vec::new(),
            ),
        };
        let reply = match outcome {
            Ok(accepted) => {
                let accepted = Box::new(accepted);
                *self = Self::Ready(accepted.clone());
                HelloReply::Accepted(accepted)
            }
            Err(refusal) => {
                *self = Self::Closed;
                HelloReply::Refused(refusal)
            }
        };
        (reply, actions)
    }

    /// The accepted hello a request is admitted under, refused before it.
    pub fn admit(&self) -> Result<&ServerHello, TransportError> {
        match self {
            Self::Ready(accepted) => Ok(accepted.as_ref()),
            Self::Awaiting => Err(TransportError::InvalidFrame {
                reason: "a request before the stream's hello".to_string(),
            }),
            Self::Closed => Err(TransportError::InvalidFrame {
                reason: "a request on a closed stream".to_string(),
            }),
        }
    }
}
