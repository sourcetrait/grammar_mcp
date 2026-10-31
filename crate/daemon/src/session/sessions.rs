//! The worker's sessions: its one lease, and each open stream by its lease.
use crate::*;
use std::collections::{HashMap, HashSet};

/// The clock a lease is timed by, in monotonic milliseconds.
pub(crate) type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// One open stream's stop, rung with why its lease ended it.
pub(crate) type StopRing = tk::sync::watch::Receiver<Option<TransportError>>;

/// A registered stream's own number, to deregister it by.
pub(crate) type StreamId = u64;

/// The worker's lease and the streams bound to it, behind one lock.
pub(crate) struct Sessions {
    inner: std::sync::Mutex<Inner>,
    clock: Clock,
}

impl std::fmt::Debug for Sessions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sessions").finish_non_exhaustive()
    }
}

/// What the lock holds.
struct Inner {
    leases: LeaseTable,
    streams: HashMap<StreamId, Bound>,
    next: StreamId,
    /// The leases admitting nothing more, which Dispatch reads.
    stopped: HashSet<LeaseId>,
    /// The epoch the Channel was last started afresh for, which EventBroker reads.
    channel_epoch: Option<LeaseEpoch>,
}

/// One open stream: the lease it holds, if any, and its stop.
struct Bound {
    lease: Option<LeaseId>,
    stop: tk::sync::watch::Sender<Option<TransportError>>,
}

impl Sessions {
    /// The sessions of `generation`'s worker, timed by `clock`.
    pub(crate) fn new(
        generation: WorkerGeneration,
        clock: Clock,
    ) -> Self {
        Self {
            inner: std::sync::Mutex::new(Inner {
                leases: LeaseTable::new(generation),
                streams: HashMap::new(),
                next: 0,
                stopped: HashSet::new(),
                channel_epoch: None,
            }),
            clock,
        }
    }

    /// `hello` answered, its lease bound and the stream registered under it at once.
    pub(crate) fn admit(
        &self,
        hello: &ClientHello,
        profile: &WorkerProfile,
        fresh: Fresh,
    ) -> (HelloReply, Option<(StreamId, StopRing)>) {
        let now = (self.clock)();
        let mut inner = self.lock();
        let mut state = HelloState::default();
        let (reply, actions) = state.on_hello(hello, profile, &mut inner.leases, now, fresh);
        inner.carry(actions);
        let registered = match &reply {
            HelloReply::Accepted(accepted) => Some(inner.register(&accepted.lease)),
            HelloReply::Refused(_) => None,
        };
        (reply, registered)
    }

    /// A controller's lease message answered, its actions carried out.
    pub(crate) fn request(
        &self,
        request: &LeaseRequest,
    ) -> Result<LeaseReply, TransportError> {
        let now = (self.clock)();
        let mut inner = self.lock();
        let change = inner.leases.request(now, request);
        inner.carry(change.actions);
        change.outcome
    }

    /// The table ticked at the clock's now, a lapsed lease's actions carried out.
    pub(crate) fn tick(&self) {
        let now = (self.clock)();
        let mut inner = self.lock();
        let actions = inner.leases.tick(now);
        inner.carry(actions);
    }

    /// A stream forgotten at its end.
    pub(crate) fn deregister(
        &self,
        id: StreamId,
    ) {
        self.lock().streams.remove(&id);
    }

    /// Whether `lease` admits nothing more.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn admission_stopped(
        &self,
        lease: &LeaseId,
    ) -> bool {
        self.lock().stopped.contains(lease)
    }

    /// The epoch the Channel was last started afresh for.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn channel_epoch(&self) -> Option<LeaseEpoch> {
        self.lock().channel_epoch
    }

    /// The streams registered, every role's.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn open_streams(&self) -> usize {
        self.lock().streams.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Inner {
    /// A stream registered under `binding`'s lease, its stop handed back.
    fn register(
        &mut self,
        binding: &LeaseBinding,
    ) -> (StreamId, StopRing) {
        let lease = match binding {
            LeaseBinding::Granted(grant) => Some(grant.lease),
            LeaseBinding::Attached { lease, .. } => Some(*lease),
            LeaseBinding::Unbound => None,
        };
        let (stop, ring) = tk::sync::watch::channel(None);
        let id = self.next;
        self.next += 1;
        self.streams.insert(id, Bound { lease, stop });
        (id, ring)
    }

    /// Each action carried out, a loss's reason given to the close it brings.
    fn carry(
        &mut self,
        actions: Vec<LeaseAction>,
    ) {
        let mut reasons = HashMap::new();
        for action in &actions {
            if let LeaseAction::Notify(LeaseNotice::Lost { lease, reason, .. }) = action {
                reasons.insert(*lease, *reason);
            }
        }
        for action in actions {
            match action {
                LeaseAction::Notify(_) => {}
                LeaseAction::StopAdmission { lease } => {
                    self.stopped.insert(lease);
                }
                LeaseAction::CloseStreams { lease } => {
                    let why = loss_error(reasons.get(&lease).copied());
                    for bound in self.streams.values().filter(|bound| bound.lease == Some(lease)) {
                        let _ = bound.stop.send(Some(why.clone()));
                    }
                }
                LeaseAction::ResetChannel { epoch } => self.channel_epoch = Some(epoch),
            }
        }
    }
}

/// How often the table is ticked for a lapsed lease.
const TICK: std::time::Duration = std::time::Duration::from_secs(1);

/// `sessions` ticked each second until `stop` changes.
pub(crate) async fn ticker(
    sessions: Arc<Sessions>,
    mut stop: tk::sync::watch::Receiver<bool>,
) {
    let mut each = tk::time::interval(TICK);
    each.set_missed_tick_behavior(tk::time::MissedTickBehavior::Delay);
    loop {
        tk::select! {
            _ = each.tick() => sessions.tick(),
            _ = stop.changed() => break,
        }
    }
}

/// The error a lost lease's streams close with.
fn loss_error(reason: Option<LossReason>) -> TransportError {
    match reason {
        Some(LossReason::Expired) => TransportError::LeaseExpired,
        Some(LossReason::Released) => TransportError::LeaseInvalid {
            reason: "the lease was released".to_string(),
        },
        Some(LossReason::TakenOver) => TransportError::LeaseInvalid {
            reason: "the lease was taken over".to_string(),
        },
        None => TransportError::LeaseInvalid {
            reason: "the lease is gone".to_string(),
        },
    }
}
