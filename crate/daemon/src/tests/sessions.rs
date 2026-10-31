//! The sessions on an injected clock: a lease's streams stopped as it is lost.
use crate::*;
use sourcetrait_grammar_protocol::{ClientInstanceId, Expectation, LeaseGrant, LeaseIntent};
use std::sync::atomic::AtomicU64;

const GENERATION: WorkerGeneration = WorkerGeneration([9; 16]);

fn instance() -> Instance {
    Instance::new("two", "default").unwrap()
}

fn profile() -> WorkerProfile {
    WorkerProfile {
        uid: 1000,
        instance: instance(),
        generation: GENERATION,
        limits: LIMITS_DEFAULT,
        artifacts: ARTIFACT_LIMITS_DEFAULT,
        capabilities: Capabilities::NONE,
    }
}

/// Sessions on a clock the test sets, and the clock's hand.
fn sessions() -> (Sessions, Arc<AtomicU64>) {
    let now = Arc::new(AtomicU64::new(0));
    let hand = now.clone();
    (Sessions::new(GENERATION, Arc::new(move || hand.load(Ordering::SeqCst))), now)
}

fn hello(role: StreamRole, lease: LeaseIntent) -> ClientHello {
    let expect = Expectation {
        uid: 1000,
        instance: instance(),
        role,
        lease,
        requested: LIMITS_DEFAULT,
        required: Vec::new(),
        last: None,
    };
    expect.hello(ClientInstanceId([1; 16]))
}

fn fresh(n: u8) -> Fresh {
    Fresh { lease: LeaseId([n; 16]), token: LeaseToken([n; 32]) }
}

/// A claim or a takeover's grant and its stream's stop.
fn granted(sessions: &Sessions, intent: LeaseIntent, n: u8) -> (LeaseGrant, StopRing) {
    let (reply, registered) = sessions.admit(&hello(StreamRole::Rpc, intent), &profile(), fresh(n));
    let HelloReply::Accepted(accepted) = reply else { panic!("granted: {reply:?}") };
    let LeaseBinding::Granted(grant) = accepted.lease else { panic!("a grant: {:?}", accepted.lease) };
    (grant, registered.expect("registered").1)
}

fn attach(sessions: &Sessions, grant: &LeaseGrant) -> StopRing {
    let intent = LeaseIntent::Attach { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation: GENERATION };
    let (reply, registered) = sessions.admit(&hello(StreamRole::Events, intent), &profile(), fresh(0));
    assert!(matches!(reply, HelloReply::Accepted(_)), "attached: {reply:?}");
    registered.expect("registered").1
}

fn rung(ring: &StopRing) -> Option<TransportError> {
    ring.borrow().clone()
}

#[test]
fn an_expired_lease_stops_its_streams() {
    let (sessions, now) = sessions();
    let (grant, rpc) = granted(&sessions, LeaseIntent::Claim, 1);
    let events = attach(&sessions, &grant);
    now.store(u64::from(grant.expiry_ms) + 1, Ordering::SeqCst);
    sessions.tick();
    assert!(matches!(rung(&rpc), Some(TransportError::LeaseExpired)), "{:?}", rung(&rpc));
    assert!(matches!(rung(&events), Some(TransportError::LeaseExpired)), "{:?}", rung(&events));
    assert!(sessions.admission_stopped(&grant.lease), "nothing more is admitted under it");
}

#[test]
fn a_heartbeat_holds_its_lease_past_the_first_expiry() {
    let (sessions, now) = sessions();
    let (grant, rpc) = granted(&sessions, LeaseIntent::Claim, 1);
    now.store(u64::from(grant.expiry_ms) - 1, Ordering::SeqCst);
    let renewed = sessions.request(&LeaseRequest::Heartbeat { lease: grant.lease, token: grant.token });
    assert!(matches!(renewed, Ok(LeaseReply::Renewed { .. })), "{renewed:?}");
    now.store(u64::from(grant.expiry_ms) + 1, Ordering::SeqCst);
    sessions.tick();
    assert!(rung(&rpc).is_none(), "the renewed lease lives");
    assert!(!sessions.admission_stopped(&grant.lease));
}

#[test]
fn a_takeover_stops_the_old_leases_streams_alone() {
    let (sessions, _now) = sessions();
    let (old, old_rpc) = granted(&sessions, LeaseIntent::Claim, 1);
    let old_events = attach(&sessions, &old);
    let (new, new_rpc) = granted(&sessions, LeaseIntent::Takeover, 2);
    assert_ne!(new.lease, old.lease);
    assert!(new.epoch.0 > old.epoch.0, "the epoch moves on");
    for ring in [&old_rpc, &old_events] {
        assert!(matches!(rung(ring), Some(TransportError::LeaseInvalid { reason }) if reason.contains("taken over")));
    }
    assert!(rung(&new_rpc).is_none(), "the new lease's stream lives");
    assert_eq!(sessions.channel_epoch(), Some(new.epoch), "the Channel starts afresh for the new epoch");
}

#[test]
fn a_release_stops_its_streams_and_frees_the_claim() {
    let (sessions, _now) = sessions();
    let (grant, rpc) = granted(&sessions, LeaseIntent::Claim, 1);
    let released = sessions.request(&LeaseRequest::Release { lease: grant.lease, token: grant.token });
    assert!(matches!(released, Ok(LeaseReply::Released)), "{released:?}");
    assert!(matches!(rung(&rpc), Some(TransportError::LeaseInvalid { reason }) if reason.contains("released")));
    let (again, _) = granted(&sessions, LeaseIntent::Claim, 3);
    assert_ne!(again.lease, grant.lease, "a fresh claim is granted");
}

#[test]
fn a_claim_beside_a_live_lease_is_refused_busy() {
    let (sessions, _now) = sessions();
    let _held = granted(&sessions, LeaseIntent::Claim, 1);
    let (reply, registered) = sessions.admit(&hello(StreamRole::Rpc, LeaseIntent::Claim), &profile(), fresh(2));
    assert!(matches!(reply, HelloReply::Refused(TransportError::Busy)), "{reply:?}");
    assert!(registered.is_none(), "a refused stream is never registered");
    assert_eq!(sessions.open_streams(), 1);
}
