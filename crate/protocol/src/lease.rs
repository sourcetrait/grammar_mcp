//! A namespace's controller lease: its messages, intents, and the worker's table.
use crate::*;

/// How often a controller heartbeats its lease.
pub const LEASE_HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(10);

/// How long a lease lives past its last grant or heartbeat.
pub const LEASE_EXPIRY: std::time::Duration = std::time::Duration::from_secs(30);

/// A lease's opaque proof of ownership, drawn from the OS's CSPRNG.
#[cereal::derived(Data, Copy, Eq, Hash, not(Debug))]
pub struct LeaseToken(pub [u8; 32]);

impl fmt::Debug for LeaseToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LeaseToken(redacted)")
    }
}

impl LeaseToken {
    /// Whether `other` is this token, compared in time its bytes don't steer.
    pub fn matches(&self, other: &LeaseToken) -> bool {
        self.0.iter().zip(other.0.iter()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    }
}

/// A lease's epoch, raised at each grant.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct LeaseEpoch(pub u64);

/// What a stream's hello asks of the namespace's lease.
#[cereal::derived(Data, Eq)]
pub enum LeaseIntent {
    /// A new lease, refused while another lives.
    Claim,
    /// The client's own live lease again, on a new RPC stream.
    Resume {
        lease: LeaseId,
        token: LeaseToken,
        epoch: LeaseEpoch,
        generation: WorkerGeneration,
    },
    /// A new lease, ending any live one.
    Takeover,
    /// An Events stream bound to the live lease.
    Attach {
        lease: LeaseId,
        token: LeaseToken,
        epoch: LeaseEpoch,
        generation: WorkerGeneration,
    },
    /// A Status stream, which holds no lease.
    Observe,
}

impl LeaseIntent {
    /// Whether a stream of `role` may carry this intent.
    pub fn fits(&self, role: StreamRole) -> bool {
        matches!(
            (role, self),
            (StreamRole::Rpc, Self::Claim | Self::Resume { .. } | Self::Takeover)
                | (StreamRole::Events, Self::Attach { .. })
                | (StreamRole::Status, Self::Observe)
        )
    }

    /// The intent's name, as a refusal reports it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Resume { .. } => "resume",
            Self::Takeover => "takeover",
            Self::Attach { .. } => "attach",
            Self::Observe => "observe",
        }
    }
}

/// A granted lease, as its controller holds it.
#[cereal::derived(Data, Eq)]
pub struct LeaseGrant {
    pub lease: LeaseId,
    pub token: LeaseToken,
    pub epoch: LeaseEpoch,
    pub heartbeat_ms: u32,
    pub expiry_ms: u32,
}

/// The lease a stream is bound to, as its hello's reply names it.
#[cereal::derived(Data, Eq)]
pub enum LeaseBinding {
    /// An RPC stream's lease, granted or resumed.
    Granted(LeaseGrant),
    /// An Events stream's lease, its token never sent back.
    Attached { lease: LeaseId, epoch: LeaseEpoch },
    /// A Status stream's, which holds none.
    Unbound,
}

/// A controller's lease message on its RPC stream.
#[cereal::derived(Data, Eq)]
pub enum LeaseRequest {
    Heartbeat { lease: LeaseId, token: LeaseToken },
    Release { lease: LeaseId, token: LeaseToken },
}

/// The worker's answer to a lease message.
#[cereal::derived(Data, Eq)]
pub enum LeaseReply {
    Renewed { epoch: LeaseEpoch },
    Released,
}

/// Why a lease was lost.
#[cereal::derived(Data, Copy, Eq)]
pub enum LossReason {
    Expired,
    Released,
    TakenOver,
}

/// A lease's change as its Events stream carries it.
#[cereal::derived(Data, Eq)]
pub enum LeaseNotice {
    Lost {
        lease: LeaseId,
        epoch: LeaseEpoch,
        reason: LossReason,
    },
}

/// What the runtime does as a lease changes, none of it to an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseAction {
    /// Tell the lost lease's Events stream why.
    Notify(LeaseNotice),
    /// Admit nothing more under the lease.
    StopAdmission { lease: LeaseId },
    /// Close every stream bound to the lease.
    CloseStreams { lease: LeaseId },
    /// Start the Channel afresh for a new epoch, nothing verified carried over.
    ResetChannel { epoch: LeaseEpoch },
}

/// What a grant draws: a fresh lease id and token, from the OS's CSPRNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fresh {
    pub lease: LeaseId,
    pub token: LeaseToken,
}

/// A change to the table: its outcome and the actions it takes.
#[derive(Debug, Clone, PartialEq)]
pub struct Change<T> {
    pub outcome: Result<T, TransportError>,
    pub actions: Vec<LeaseAction>,
}

/// The lease that lives.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Live {
    lease: LeaseId,
    token: LeaseToken,
    epoch: LeaseEpoch,
    /// The moment it lapses, in the caller's monotonic milliseconds.
    deadline_ms: u64,
}

/// A worker's one lease, held by at most one controller at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseTable {
    generation: WorkerGeneration,
    /// The last epoch granted.
    epoch: LeaseEpoch,
    live: Option<Live>,
    /// The last lease lost, so a late message for it learns why.
    lost: Option<(LeaseId, LossReason)>,
}

/// A duration's milliseconds, saturating.
fn millis(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

impl LeaseTable {
    /// A worker's table at its start, no lease granted.
    pub fn new(generation: WorkerGeneration) -> Self {
        Self { generation, epoch: LeaseEpoch(0), live: None, lost: None }
    }

    /// The live lease's id and epoch, if one lives at `now_ms`.
    pub fn live(&self, now_ms: u64) -> Option<(LeaseId, LeaseEpoch)> {
        self.live.as_ref().filter(|live| now_ms < live.deadline_ms).map(|live| (live.lease, live.epoch))
    }

    /// The live lease lost for `reason`, and the actions its loss takes.
    fn lose(&mut self, reason: LossReason) -> Vec<LeaseAction> {
        let Some(live) = self.live.take() else { return Vec::new() };
        self.lost = Some((live.lease, reason));
        vec![
            LeaseAction::Notify(LeaseNotice::Lost { lease: live.lease, epoch: live.epoch, reason }),
            LeaseAction::StopAdmission { lease: live.lease },
            LeaseAction::CloseStreams { lease: live.lease },
        ]
    }

    /// A lease past its deadline at `now_ms` lost, and its loss's actions.
    pub fn tick(&mut self, now_ms: u64) -> Vec<LeaseAction> {
        match &self.live {
            Some(live) if now_ms >= live.deadline_ms => self.lose(LossReason::Expired),
            _ => Vec::new(),
        }
    }

    /// A new lease at a new epoch, the Channel reset for it.
    fn grant(&mut self, now_ms: u64, fresh: Fresh, actions: &mut Vec<LeaseAction>) -> LeaseGrant {
        self.epoch = LeaseEpoch(self.epoch.0 + 1);
        self.live = Some(Live {
            lease: fresh.lease,
            token: fresh.token,
            epoch: self.epoch,
            deadline_ms: now_ms.saturating_add(millis(LEASE_EXPIRY)),
        });
        actions.push(LeaseAction::ResetChannel { epoch: self.epoch });
        self.grant_of(self.live.as_ref().expect("granted"))
    }

    fn grant_of(&self, live: &Live) -> LeaseGrant {
        LeaseGrant {
            lease: live.lease,
            token: live.token,
            epoch: live.epoch,
            heartbeat_ms: u32::try_from(millis(LEASE_HEARTBEAT)).unwrap_or(u32::MAX),
            expiry_ms: u32::try_from(millis(LEASE_EXPIRY)).unwrap_or(u32::MAX),
        }
    }

    /// The live lease `lease` names with its token, or refused.
    fn holder(&mut self, lease: LeaseId, token: &LeaseToken) -> Result<&mut Live, TransportError> {
        let lost = self.lost;
        match self.live.as_mut() {
            Some(live) if live.lease == lease && live.token.matches(token) => Ok(live),
            Some(live) if live.lease == lease => Err(TransportError::LeaseInvalid {
                reason: "the token is not the lease's".to_string(),
            }),
            _ => match lost {
                Some((gone, LossReason::Expired)) if gone == lease => Err(TransportError::LeaseExpired),
                Some((gone, reason)) if gone == lease => Err(TransportError::LeaseInvalid {
                    reason: format!("the lease was {}", match reason {
                        LossReason::Released => "released",
                        LossReason::TakenOver => "taken over",
                        LossReason::Expired => "expired",
                    }),
                }),
                _ => Err(TransportError::LeaseInvalid { reason: format!("no lease {}", lease.hex()) }),
            },
        }
    }

    /// A stream's hello intent bound at `now_ms`, `fresh` drawn for a grant.
    pub fn bind(&mut self, now_ms: u64, role: StreamRole, intent: &LeaseIntent, fresh: Fresh) -> Change<LeaseBinding> {
        let mut actions = self.tick(now_ms);
        let outcome = self.bind_ticked(now_ms, role, intent, fresh, &mut actions);
        Change { outcome, actions }
    }

    fn bind_ticked(
        &mut self,
        now_ms: u64,
        role: StreamRole,
        intent: &LeaseIntent,
        fresh: Fresh,
        actions: &mut Vec<LeaseAction>,
    ) -> Result<LeaseBinding, TransportError> {
        if !intent.fits(role) {
            return Err(TransportError::InvalidFrame {
                reason: format!("a {role:?} stream can't {}", intent.name()),
            });
        }
        match intent {
            LeaseIntent::Observe => Ok(LeaseBinding::Unbound),
            LeaseIntent::Claim if self.live.is_some() => Err(TransportError::Busy),
            LeaseIntent::Claim => Ok(LeaseBinding::Granted(self.grant(now_ms, fresh, actions))),
            LeaseIntent::Takeover => {
                actions.extend(self.lose(LossReason::TakenOver));
                Ok(LeaseBinding::Granted(self.grant(now_ms, fresh, actions)))
            }
            LeaseIntent::Resume { lease, token, epoch, generation }
            | LeaseIntent::Attach { lease, token, epoch, generation } => {
                if *generation != self.generation {
                    return Err(TransportError::LeaseInvalid {
                        reason: "the lease belongs to another worker generation".to_string(),
                    });
                }
                let resume = matches!(intent, LeaseIntent::Resume { .. });
                let live = self.holder(*lease, token)?;
                if live.epoch != *epoch {
                    return Err(TransportError::LeaseInvalid {
                        reason: format!("epoch {} where the lease's is {}", epoch.0, live.epoch.0),
                    });
                }
                if resume {
                    live.deadline_ms = now_ms.saturating_add(millis(LEASE_EXPIRY));
                    let live = live.clone();
                    Ok(LeaseBinding::Granted(self.grant_of(&live)))
                } else {
                    Ok(LeaseBinding::Attached { lease: live.lease, epoch: live.epoch })
                }
            }
        }
    }

    /// A controller's lease message at `now_ms`, and every action it takes.
    pub fn request(&mut self, now_ms: u64, request: &LeaseRequest) -> Change<LeaseReply> {
        let mut actions = self.tick(now_ms);
        let outcome = match request {
            LeaseRequest::Heartbeat { lease, token } => self.holder(*lease, token).map(|live| {
                live.deadline_ms = now_ms.saturating_add(millis(LEASE_EXPIRY));
                LeaseReply::Renewed { epoch: live.epoch }
            }),
            LeaseRequest::Release { lease, token } => {
                let held = self.holder(*lease, token).map(|_| ());
                held.map(|()| {
                    actions.extend(self.lose(LossReason::Released));
                    LeaseReply::Released
                })
            }
        };
        Change { outcome, actions }
    }
}
