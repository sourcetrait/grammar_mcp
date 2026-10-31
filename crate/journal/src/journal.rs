//! The journal: the protocol's operation model, each move durable before it holds.
use crate::*;

/// The bytes a journal's segments and sidecars may hold before it admits no more.
pub const CAP_DEFAULT: u64 = 1024 * 1024 * 1024;

/// The most bytes of an outcome kept in its entry; a larger one gets a sidecar.
pub const INLINE_MAX: usize = 64 * 1024;

/// How many times a ticket's ids are drawn while the model holds them.
const DRAWS: usize = 4;

/// The bounds a kept outcome reads back within: any a client could take.
const OUTCOME_BUDGET: DecodeBudget = DecodeBudget { heap: DECODE_HEAP_HARD, ..DECODE_BUDGET_DEFAULT };

/// What a journal opens on: its directory, its worker, and its cap.
#[derive(Debug, Clone)]
pub struct JournalSpec {
    pub root: PathBuf,
    pub generation: WorkerGeneration,
    pub instance: Instance,
    pub cap: u64,
}

/// Where a finished operation's outcome is kept.
#[derive(Debug, Clone)]
enum Kept {
    Inline { offset: u64, len: u64 },
    Sidecar { sha256: String },
}

/// A worker's operation journal: its model, its segment, and its sidecars.
pub struct Journal {
    dir: JournalDir,
    segment: Segment,
    table: OperationTable,
    kept: HashMap<OperationId, Kept>,
    reserved: HashMap<OperationId, u64>,
    cap: u64,
    used: u64,
    degraded: Option<String>,
    faults: Faults,
}

impl Journal {
    /// The journal at `spec`'s root: its lock taken, its segment and header made.
    pub fn open(spec: JournalSpec, faults: Faults) -> JournalResult<Self> {
        let opened = JournalDir::open(&spec.root)?;
        let segments = opened.dir.segments();
        let segment = Segment::create(&segments, opened.next)
            .map_err(|e| JournalError::Unavailable { path: segments, reason: format!("its segment: {e}") })?;
        let mut journal = Self {
            dir: opened.dir,
            segment,
            table: OperationTable::new(spec.generation),
            kept: HashMap::new(),
            reserved: HashMap::new(),
            cap: spec.cap,
            used: opened.used,
            degraded: None,
            faults,
        };
        let header = Entry::Opened {
            format: FORMAT,
            generation: spec.generation.hex(),
            instance: spec.instance.to_string(),
            pid: std::process::id(),
            at: wall_ms(),
        };
        match journal.segment.append(header, &mut journal.faults) {
            Ok((_, len)) => journal.used += len,
            Err(e) => {
                let path = journal.segment.path().to_path_buf();
                return Err(JournalError::Unavailable { path, reason: format!("its header: {e}") });
            }
        }
        Ok(journal)
    }

    /// A ticket under the lease's `epoch`, its ids drawn by `draw` until fresh.
    pub fn issue(
        &mut self,
        now_ms: u64,
        epoch: LeaseEpoch,
        draw: &mut dyn FnMut() -> io::Result<(OperationId, datum::Nonce)>,
    ) -> Result<OperationTicket, TransportError> {
        if let Some(reason) = &self.degraded {
            return Err(unrecorded(reason));
        }
        for _ in 0..DRAWS {
            let (operation, nonce) = draw().map_err(|e| TransportError::Unavailable {
                reason: format!("no ticket's ids can be drawn: {e}"),
            })?;
            if !self.table.holds(operation, nonce) {
                return self.table.issue(now_ms, operation, nonce, epoch);
            }
        }
        Err(TransportError::Unavailable { reason: "every id drawn for a ticket was held".to_string() })
    }

    /// `submission` under the lease's `epoch`, admitted once its entry is synced.
    pub fn submit(&mut self, now_ms: u64, submission: &Submission, epoch: LeaseEpoch, reserve: u64) -> Submitted {
        let operation = submission.ticket.operation;
        if let Some(admission) = self.table.check(now_ms, submission, epoch) {
            let response = match &admission.outcome {
                AdmissionOutcome::Existing(OperationStatus::Known { state, .. }) if state.terminal() => {
                    self.response(submission.correlation, operation).map(Box::new)
                }
                _ => None,
            };
            return Submitted { admission, grant: None, response };
        }
        let refused = |error| Submitted {
            admission: Admission {
                correlation: submission.correlation,
                operation,
                outcome: AdmissionOutcome::Refused(error),
            },
            grant: None,
            response: None,
        };
        if let Some(reason) = &self.degraded {
            return refused(unrecorded(reason));
        }
        if let Some(reason) = self.short_of_room(reserve) {
            return refused(TransportError::StorageFull { reason });
        }
        let entry = Entry::Admitted {
            operation: operation.hex(),
            nonce: submission.ticket.nonce.to_string(),
            hash: hex(&submission.hash),
            epoch: submission.ticket.epoch.0,
            at: wall_ms(),
        };
        if let Err(error) = self.append(entry) {
            let reason = match error {
                StepError::Degraded { reason } => reason,
                other => other.to_string(),
            };
            return refused(unrecorded(&reason));
        }
        let (admission, execute) = self.table.submit(now_ms, submission, epoch);
        self.reserved.insert(operation, reserve);
        let grant = execute.map(|execute| Grant { operation: execute.operation, nonce: execute.nonce });
        Submitted { admission, grant, response: None }
    }

    /// `operation` moved by `step` once its entry is synced: its new state.
    pub fn advance(&mut self, operation: OperationId, step: Step) -> Result<OperationState, StepError> {
        let (state, next) = self.moving(operation, step.event())?;
        if next == state {
            return Ok(next);
        }
        let at = wall_ms();
        let entry = match step {
            Step::Start => Entry::Running { operation: operation.hex(), at },
            Step::RequestCancel => Entry::CancelRequested { operation: operation.hex(), at },
        };
        self.append(entry)?;
        self.table.advance(operation, step.event());
        Ok(next)
    }

    /// `operation` ended by `ending` once its outcome and entry are synced.
    pub fn finish(
        &mut self,
        operation: OperationId,
        ending: Ending,
        outcome: &Result<RuntimeReply, TransportError>,
    ) -> Result<OperationState, StepError> {
        let (_, next) = self.moving(operation, ending.event())?;
        let bytes = bitcode::encode(outcome);
        let (stored, sidecar) = if bytes.len() <= INLINE_MAX {
            (Stored::Inline(Blob(bytes)), None)
        } else {
            let (sha256, added) = match put_sidecar(&self.dir.results(), &bytes, &mut self.faults) {
                Ok(kept) => kept,
                Err(e) => return Err(self.degrade(e)),
            };
            self.used = self.used.saturating_add(added);
            (Stored::Sidecar { sha256: sha256.clone(), bytes: bytes.len() as u64 }, Some(sha256))
        };
        let entry = Entry::Finished { operation: operation.hex(), state: next, at: wall_ms(), outcome: stored };
        let (offset, len) = self.append(entry)?;
        self.table.advance(operation, ending.event());
        let kept = match sidecar {
            Some(sha256) => Kept::Sidecar { sha256 },
            None => Kept::Inline { offset, len },
        };
        self.kept.insert(operation, kept);
        self.reserved.remove(&operation);
        Ok(next)
    }

    /// What is known of `operation`, at its last durable state.
    pub fn lookup(&self, operation: OperationId) -> OperationStatus {
        self.table.lookup(operation)
    }

    /// A finished operation's kept outcome, read back; none where none is kept.
    pub fn outcome(&self, operation: OperationId) -> Option<io::Result<Result<RuntimeReply, TransportError>>> {
        let kept = self.kept.get(&operation)?;
        let bytes = match kept {
            Kept::Inline { offset, len } => match self.segment.read(*offset, *len).map(|line| line.entry) {
                Ok(Entry::Finished { outcome: Stored::Inline(Blob(bytes)), .. }) => Ok(bytes),
                Ok(_) => Err(invalid("the line kept for an outcome holds none")),
                Err(e) => Err(e),
            },
            Kept::Sidecar { sha256 } => read_sidecar(&self.dir.results(), sha256),
        };
        Some(bytes.and_then(|bytes| decode_payload(&bytes, OUTCOME_BUDGET).map_err(invalid)))
    }

    /// Why the journal records nothing more, where it doesn't.
    pub fn degraded(&self) -> Option<&str> {
        self.degraded.as_deref()
    }

    /// The bytes its segments and sidecars hold.
    pub fn used(&self) -> u64 {
        self.used
    }

    /// The journal's directory.
    pub fn root(&self) -> &Path {
        self.dir.root()
    }

    /// The segment this journal writes.
    pub fn segment(&self) -> &Path {
        self.segment.path()
    }

    /// `operation`'s state and the one `event` moves it to, or why there's none.
    fn moving(&self, operation: OperationId, event: OperationEvent) -> Result<(OperationState, OperationState), StepError> {
        if let Some(reason) = &self.degraded {
            return Err(StepError::Degraded { reason: reason.clone() });
        }
        let OperationStatus::Known { state, .. } = self.table.lookup(operation) else {
            return Err(StepError::Refused { operation, state: None });
        };
        let next = transition(state, event).ok_or(StepError::Refused { operation, state: Some(state) })?;
        Ok((state, next))
    }

    /// `entry` written and synced, the journal degraded where that fails.
    fn append(&mut self, entry: Entry) -> Result<(u64, u64), StepError> {
        match self.segment.append(entry, &mut self.faults) {
            Ok((offset, len)) => {
                self.used = self.used.saturating_add(len);
                Ok((offset, len))
            }
            Err(e) => Err(self.degrade(e)),
        }
    }

    /// The journal stopped recording at `error`, said once.
    fn degrade(&mut self, error: io::Error) -> StepError {
        let reason = self.degraded.get_or_insert_with(|| {
            let reason = error.to_string();
            eprintln!("grammar: the operation journal records nothing more: {reason}");
            reason
        });
        StepError::Degraded { reason: reason.clone() }
    }

    /// Why a fresh admission has no room, where it hasn't.
    fn short_of_room(&self, reserve: u64) -> Option<String> {
        if self.used >= self.cap {
            return Some(format!("the journal holds {} bytes against its cap of {}", self.used, self.cap));
        }
        let needed = self.reserved.values().fold(reserve, |sum, held| sum.saturating_add(*held));
        match free_bytes(self.dir.root()) {
            Ok(free) if free >= needed => None,
            Ok(free) => Some(format!("the filesystem has {free} bytes free where {needed} are held for operations")),
            Err(e) => Some(format!("the filesystem's free space can't be read: {e}")),
        }
    }

    /// A finished operation's outcome as its duplicate's response.
    fn response(&self, correlation: u64, operation: OperationId) -> Option<Response> {
        let outcome = match self.outcome(operation)? {
            Ok(outcome) => outcome,
            Err(e) => Err(TransportError::Unavailable { reason: format!("its kept outcome can't be read: {e}") }),
        };
        Some(Response { correlation, operation, outcome })
    }
}

/// The refusal of a journal that records nothing more; nothing ran.
fn unrecorded(reason: &str) -> TransportError {
    TransportError::Unavailable { reason: format!("the operation journal records nothing more: {reason}") }
}

/// The bytes free to this user on the filesystem holding `path`.
fn free_bytes(path: &Path) -> io::Result<u64> {
    let stat = nix::sys::statvfs::statvfs(path)?;
    Ok((stat.blocks_available() as u64).saturating_mul(stat.fragment_size() as u64))
}

/// Milliseconds since the Unix epoch by the wall clock, for retention alone.
fn wall_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
