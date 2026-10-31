//! An operation's ticket, admission, status, and outcome, and the worker's model.
use crate::*;
use std::collections::HashMap;

/// How long an issued ticket waits for its submission.
pub const TICKET_EXPIRY: std::time::Duration = std::time::Duration::from_secs(60);

/// The most tickets awaiting submission at once.
pub const TICKETS_MAX: usize = 1024;

/// An admitted operation's state.
#[cereal::derived(Data, Copy, Eq)]
pub enum OperationState {
    Admitted,
    Running,
    Succeeded,
    Failed,
    CancelRequested,
    Cancelled,
    /// Admitted, running, or cancelling when its worker stopped: never run again.
    UnknownAfterRestart,
}

impl OperationState {
    /// Whether no event moves the state again.
    pub fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled | Self::UnknownAfterRestart)
    }
}

/// What a lookup knows of an operation.
#[cereal::derived(Data)]
pub enum OperationStatus {
    Known {
        operation: OperationId,
        state: OperationState,
    },
    Unknown {
        operation: OperationId,
    },
    /// Its result no longer kept, with the terminal state it reached where known.
    Expired {
        operation: OperationId,
        terminal: Option<OperationState>,
    },
}

/// What moves an operation's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationEvent {
    Dispatch,
    Succeed,
    Fail,
    RequestCancel,
    Cancelled,
    Restart,
}

/// The state `event` moves `state` to, or none where the contract has no such move.
pub fn transition(state: OperationState, event: OperationEvent) -> Option<OperationState> {
    use OperationEvent as E;
    use OperationState as S;
    match (state, event) {
        (S::Admitted, E::Dispatch) => Some(S::Running),
        (S::Admitted | S::Running | S::CancelRequested, E::Fail) => Some(S::Failed),
        (S::Running | S::CancelRequested, E::Succeed) => Some(S::Succeeded),
        (S::Admitted | S::Running | S::CancelRequested, E::RequestCancel) => Some(S::CancelRequested),
        (S::CancelRequested, E::Cancelled) => Some(S::Cancelled),
        (S::Admitted | S::Running | S::CancelRequested, E::Restart) => Some(S::UnknownAfterRestart),
        _ => None,
    }
}

/// Whether a request may run anything: a query is safe to repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestClass {
    Query,
    Mutating,
}

/// An `interface` command's class by its kind, a read-only command a query.
fn command_class(kind: CommandKind) -> RequestClass {
    use CommandKind as K;
    match kind {
        K::Info | K::Help | K::ProcessList | K::PurviewList | K::RemoteList | K::RemoteGet | K::EquipCheck => {
            RequestClass::Query
        }
        K::ProcessKill
        | K::PurviewSet
        | K::PurviewExtend
        | K::PurviewConfigure
        | K::ChannelOpen
        | K::ChannelSync
        | K::ChannelReady
        | K::ChannelClose
        | K::ChannelConfig
        | K::RemoteSet
        | K::RemoteUnset
        | K::RemoteListen
        | K::RemoteConnect
        | K::RemoteClose
        | K::EquipNew
        | K::EquipInstall
        | K::EquipUninstall
        | K::EquipScaffold
        | K::EquipCommit => RequestClass::Mutating,
    }
}

/// The command an `interface` text names by its longest run of leading words.
fn named_command(text: &str) -> Option<CommandKind> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let words = match words.first() {
        Some(&"interface") => &words[1..],
        _ => &words[..],
    };
    (1..=words.len()).rev().find_map(|n| command_by_words(&words[..n].join(" ")).map(|spec| spec.kind))
}

/// A request's class, decided for every variant with no default.
pub fn request_class(request: &RuntimeRequest) -> RequestClass {
    match request {
        RuntimeRequest::Engine(engine) => match engine {
            ToEngineSys::Execute(_) | ToEngineSys::Rerun(_) | ToEngineSys::Call(_) => RequestClass::Mutating,
            ToEngineSys::Control(control) => match &control.command {
                EngineCommand::Processes
                | EngineCommand::Health
                | EngineCommand::EquipCheck { .. }
                | EngineCommand::Signatures { .. }
                | EngineCommand::Help { .. } => RequestClass::Query,
                EngineCommand::Kill { .. }
                | EngineCommand::Teardown
                | EngineCommand::EquipNew { .. }
                | EngineCommand::EquipInstall { .. }
                | EngineCommand::EquipUninstall { .. }
                | EngineCommand::EquipScaffold { .. }
                | EngineCommand::EquipCommit { .. } => RequestClass::Mutating,
            },
        },
        RuntimeRequest::Interface { command } => named_command(command).map(command_class).unwrap_or(RequestClass::Mutating),
        RuntimeRequest::Info | RuntimeRequest::OperationLookup { .. } => RequestClass::Query,
        RuntimeRequest::Policy(_) | RuntimeRequest::Channel(_) | RuntimeRequest::Cancel { .. } => RequestClass::Mutating,
        RuntimeRequest::Artifact(artifact) => match artifact {
            ArtifactRequest::Inspect(_) | ArtifactRequest::Describe(_) | ArtifactRequest::ReadLog(_) => {
                RequestClass::Query
            }
            ArtifactRequest::Reserve(_) | ArtifactRequest::Publish(_) | ArtifactRequest::Abort(_) => {
                RequestClass::Mutating
            }
        },
    }
}

/// A request's operation hash: the SHA-256 of its typed value re-encoded.
pub fn payload_hash(request: &RuntimeRequest) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bitcode::encode(request)).into()
}

/// The public engine nonce an engine call carries, which its ticket must mint.
fn request_nonce(request: &RuntimeRequest) -> Option<&datum::Nonce> {
    match request {
        RuntimeRequest::Engine(ToEngineSys::Execute(r)) => Some(&r.nonce),
        RuntimeRequest::Engine(ToEngineSys::Rerun(r)) => Some(&r.nonce),
        RuntimeRequest::Engine(ToEngineSys::Call(r)) => Some(&r.nonce),
        _ => None,
    }
}

/// An operation's ticket: what the worker allots before a submission may run.
#[cereal::derived(Data, Eq)]
pub struct OperationTicket {
    pub operation: OperationId,
    /// The public engine nonce the worker minted, which the request carries.
    pub nonce: datum::Nonce,
    pub generation: WorkerGeneration,
    pub epoch: LeaseEpoch,
    /// How long the ticket waits for its submission.
    pub expires_ms: u32,
}

/// A stream's request for a ticket, which executes nothing.
#[cereal::derived(Data, Copy, Eq)]
pub struct TicketRequest {
    pub correlation: u64,
}

/// A ticket allotted, or refused.
#[cereal::derived(Data)]
pub struct TicketReply {
    pub correlation: u64,
    pub outcome: Result<OperationTicket, TransportError>,
}

/// A request submitted under its ticket, with its payload's hash.
#[cereal::derived(Data)]
pub struct Submission {
    /// The stream's correlation, never the operation's identity.
    pub correlation: u64,
    pub ticket: OperationTicket,
    pub hash: [u8; 32],
    pub request: RuntimeRequest,
}

/// What a submission came to.
#[cereal::derived(Data)]
pub enum AdmissionOutcome {
    Admitted,
    /// The operation was admitted before, with this same payload.
    Existing(OperationStatus),
    Refused(TransportError),
}

/// A submission's answer.
#[cereal::derived(Data)]
pub struct Admission {
    pub correlation: u64,
    pub operation: OperationId,
    pub outcome: AdmissionOutcome,
}

/// An operation's status asked for, which executes nothing.
#[cereal::derived(Data, Copy, Eq)]
pub struct StatusQuery {
    pub correlation: u64,
    pub operation: OperationId,
}

/// A status lookup's answer.
#[cereal::derived(Data)]
pub struct StatusReply {
    pub correlation: u64,
    pub status: OperationStatus,
}

/// An operation's terminal result, or the error that ended it.
#[cereal::derived(Data)]
pub struct Response {
    pub correlation: u64,
    pub operation: OperationId,
    pub outcome: Result<RuntimeReply, TransportError>,
}

/// What an admission asks the runtime to do: run the operation, once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execute {
    pub operation: OperationId,
    pub nonce: datum::Nonce,
}

/// A ticket issued and awaiting its submission.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Issued {
    ticket: OperationTicket,
    deadline_ms: u64,
}

/// An admitted operation, or a retired one's compact tombstone.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    hash: [u8; 32],
    state: OperationState,
    retired: bool,
}

/// A worker's operations: tickets awaiting submission, and each one admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationTable {
    generation: WorkerGeneration,
    issued: HashMap<OperationId, Issued>,
    records: HashMap<OperationId, Record>,
}

/// A ticket refused, and why.
fn invalid(reason: &str) -> TransportError {
    TransportError::TicketInvalid { reason: reason.to_string() }
}

impl OperationTable {
    /// A worker's table at its start, nothing issued or admitted.
    pub fn new(generation: WorkerGeneration) -> Self {
        Self { generation, issued: HashMap::new(), records: HashMap::new() }
    }

    /// A worker's table from its journal's records, each unfinished one unknown.
    pub fn recover(generation: WorkerGeneration, records: impl IntoIterator<Item = (OperationId, [u8; 32], OperationState)>) -> Self {
        let records = records
            .into_iter()
            .map(|(operation, hash, state)| {
                let state = transition(state, OperationEvent::Restart).unwrap_or(state);
                (operation, Record { hash, state, retired: false })
            })
            .collect();
        Self { generation, issued: HashMap::new(), records }
    }

    /// Tickets past their deadline at `now_ms` forgotten.
    fn expire(&mut self, now_ms: u64) {
        self.issued.retain(|_, issued| now_ms < issued.deadline_ms);
    }

    /// A ticket for `operation` under the lease's `epoch`, its nonce minted.
    pub fn issue(&mut self, now_ms: u64, operation: OperationId, nonce: datum::Nonce, epoch: LeaseEpoch) -> Result<OperationTicket, TransportError> {
        self.expire(now_ms);
        if self.issued.len() >= TICKETS_MAX || self.issued.contains_key(&operation) || self.records.contains_key(&operation) {
            return Err(TransportError::Busy);
        }
        let expires = u64::try_from(TICKET_EXPIRY.as_millis()).unwrap_or(u64::MAX);
        let ticket = OperationTicket {
            operation,
            nonce,
            generation: self.generation,
            epoch,
            expires_ms: u32::try_from(expires).unwrap_or(u32::MAX),
        };
        self.issued.insert(operation, Issued { ticket: ticket.clone(), deadline_ms: now_ms.saturating_add(expires) });
        Ok(ticket)
    }

    /// `submission` under the lease's `epoch`: its admission, and any execution.
    pub fn submit(&mut self, now_ms: u64, submission: &Submission, epoch: LeaseEpoch) -> (Admission, Option<Execute>) {
        let operation = submission.ticket.operation;
        let answer = |outcome| Admission { correlation: submission.correlation, operation, outcome };
        if let Some(record) = self.records.get(&operation) {
            let outcome = if record.hash == submission.hash {
                AdmissionOutcome::Existing(self.lookup(operation))
            } else {
                AdmissionOutcome::Refused(TransportError::OperationConflict { operation })
            };
            return (answer(outcome), None);
        }
        self.expire(now_ms);
        let refusal = match self.issued.get(&operation) {
            None => Some(invalid("no ticket of this worker's awaits that operation")),
            Some(issued) if issued.ticket != submission.ticket => Some(invalid("the ticket is not the one issued")),
            Some(_) if submission.ticket.generation != self.generation => Some(invalid("the ticket is another worker generation's")),
            Some(_) if submission.ticket.epoch != epoch => Some(invalid("the ticket is another lease epoch's")),
            Some(_) if request_nonce(&submission.request).is_some_and(|nonce| *nonce != submission.ticket.nonce) => {
                Some(invalid("the request's nonce is not the ticket's"))
            }
            Some(_) if payload_hash(&submission.request) != submission.hash => Some(TransportError::InvalidFrame {
                reason: "the payload's hash is not its request's".to_string(),
            }),
            Some(_) => None,
        };
        if let Some(refusal) = refusal {
            return (answer(AdmissionOutcome::Refused(refusal)), None);
        }
        let ticket = self.issued.remove(&operation).expect("checked").ticket;
        self.records.insert(operation, Record { hash: submission.hash, state: OperationState::Admitted, retired: false });
        (answer(AdmissionOutcome::Admitted), Some(Execute { operation, nonce: ticket.nonce }))
    }

    /// What is known of `operation`, which runs nothing.
    pub fn lookup(&self, operation: OperationId) -> OperationStatus {
        match self.records.get(&operation) {
            Some(record) if record.retired => OperationStatus::Expired { operation, terminal: Some(record.state) },
            Some(record) => OperationStatus::Known { operation, state: record.state },
            None => OperationStatus::Unknown { operation },
        }
    }

    /// `operation` moved by `event`: its new state, or none for an illegal move.
    pub fn advance(&mut self, operation: OperationId, event: OperationEvent) -> Option<OperationState> {
        let record = self.records.get_mut(&operation).filter(|record| !record.retired)?;
        let next = transition(record.state, event)?;
        record.state = next;
        Some(next)
    }

    /// A terminal operation's result let go, its tombstone kept.
    pub fn retire(&mut self, operation: OperationId) -> bool {
        match self.records.get_mut(&operation) {
            Some(record) if record.state.terminal() => {
                record.retired = true;
                true
            }
            _ => false,
        }
    }
}
