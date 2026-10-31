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

/// One plugin as a positional `[name, version]` pair (version may be null).
#[cereal::derived(Data)]
pub struct PluginInfo(pub String, pub Option<String>);

/// One configured purview: a label bound to what it puts in view.
#[cereal::derived(Data)]
pub struct PurviewRow {
    pub id: String,
    pub namepath_patterns: Vec<String>,
}

/// One `[id, namepath_patterns]` pair as the tools report it.
#[cereal::derived(Data)]
pub struct PurviewView(pub String, pub Vec<String>);

/// `interface info`'s answer: the execution host's identity, scratch, and surface.
#[cereal::derived(Data)]
pub struct InfoReply {
    pub name: String,
    pub version: String,
    pub nu_version: String,
    /// The `--id` the worker was configured with.
    pub id: String,
    pub namespace: String,
    /// The worker's nom, stable for its life: $env.EQUIP_MCP_NOM.
    pub nom: String,
    /// The worker's working directory, exported as $env.EQUIP_WORK_DIR.
    pub work_dir: String,
    /// The worker's bulk scratch directory, exported as $env.EQUIP_TMP.
    pub tmp: String,
    /// The worker's transient scratch directory, exported as $env.EQUIP_TRANSIENT.
    pub transient: String,
    pub plugins: Vec<PluginInfo>,
    /// The callable surface within the reported purview, as one block.
    pub signatures: String,
    /// The purview `signatures` was rendered for.
    pub purview: Vec<PurviewView>,
}

/// A controller's policy, frozen when it attaches and enforced before dispatch.
#[cereal::derived(Data)]
pub struct ControllerPolicy {
    /// The tools its frontend denies, by name.
    pub deny_tools: Vec<String>,
    /// The interface commands its frontend denies, by their words.
    pub deny_commands: Vec<String>,
}

/// A request to a worker's runtime.
#[cereal::derived(Data)]
pub enum RuntimeRequest {
    /// A typed engine request: an eval, a rerun, a call, or a control command.
    Engine(ToEngineSys),
    /// An `interface` command's text, parsed by the runtime against the catalog.
    Interface { command: String },
    Info,
    OperationLookup { operation: OperationId },
    Policy(ControllerPolicy),
    Channel(ChannelNotice),
    Artifact(ArtifactRequest),
    /// A cancellation of another operation, admitted as its own.
    Cancel { target: OperationId },
}

/// A worker runtime's answer.
#[cereal::derived(Data)]
pub enum RuntimeReply {
    Engine(FromEngineSys),
    /// An `interface` command's result, which the frontend renders.
    Interface { result: NuVal },
    Info(InfoReply),
    Operation(OperationStatus),
    Policy(ControllerPolicy),
    Channel(ChannelState),
    Artifact(ArtifactReply),
    /// The runtime's refusal, outside the engine.
    Refused(RuntimeError),
    /// A remote Channel command's typed result.
    Remote(RemoteResult),
}
