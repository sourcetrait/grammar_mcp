//! The controller lease: expiry at its boundary on an injected clock, a late
//! heartbeat, claims and takeovers racing, each wrong binding refused, each
//! role's intents, the token's Debug, the actions of each loss, none touching
//! an operation, and a golden vector of each message.

use std::collections::BTreeMap;

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/lease.nuon";

const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const EXPIRY_MS: u64 = 30_000;

/// The `n`th draw of a grant's id and token.
fn fresh(n: u8) -> Fresh {
    Fresh { lease: LeaseId([n; 16]), token: LeaseToken([n.wrapping_add(0x80); 32]) }
}

/// A claim granted on an RPC stream at `now_ms`, drawing `fresh(n)`.
fn claim(table: &mut LeaseTable, now_ms: u64, n: u8) -> (LeaseGrant, Vec<LeaseAction>) {
    let change = table.bind(now_ms, StreamRole::Rpc, &LeaseIntent::Claim, fresh(n));
    match change.outcome {
        Ok(LeaseBinding::Granted(grant)) => (grant, change.actions),
        other => panic!("a claim not granted: {other:?}"),
    }
}

fn heartbeat(grant: &LeaseGrant) -> LeaseRequest {
    LeaseRequest::Heartbeat { lease: grant.lease, token: grant.token }
}

fn resume(grant: &LeaseGrant) -> LeaseIntent {
    LeaseIntent::Resume { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation: GENERATION }
}

fn attach(grant: &LeaseGrant) -> LeaseIntent {
    LeaseIntent::Attach { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation: GENERATION }
}

/// A loss's actions: the notice, then admission stopped and the streams closed.
fn lost(grant: &LeaseGrant, reason: LossReason) -> Vec<LeaseAction> {
    vec![
        LeaseAction::Notify(LeaseNotice::Lost { lease: grant.lease, epoch: grant.epoch, reason }),
        LeaseAction::StopAdmission { lease: grant.lease },
        LeaseAction::CloseStreams { lease: grant.lease },
    ]
}

/// What an action touches, matched exhaustively so a new kind of action fails
/// the build here: none of them is an operation.
fn touches(action: &LeaseAction) -> &'static str {
    match action {
        LeaseAction::Notify(_) => "events",
        LeaseAction::StopAdmission { .. } => "admission",
        LeaseAction::CloseStreams { .. } => "streams",
        LeaseAction::ResetChannel { .. } => "channel",
    }
}

#[test]
fn a_claim_grants_the_first_epoch_and_resets_the_channel_for_it() {
    let mut table = LeaseTable::new(GENERATION);
    let (grant, actions) = claim(&mut table, 0, 1);
    assert_eq!(grant.epoch, LeaseEpoch(1));
    assert_eq!((grant.heartbeat_ms, grant.expiry_ms), (10_000, 30_000));
    assert_eq!(actions, vec![LeaseAction::ResetChannel { epoch: LeaseEpoch(1) }]);
    assert_eq!(table.live(0), Some((grant.lease, grant.epoch)));
}

#[test]
fn a_lease_lives_until_its_expiry_and_is_lost_at_it() {
    let mut table = LeaseTable::new(GENERATION);
    let (grant, _) = claim(&mut table, 1_000, 1);
    assert!(table.tick(1_000 + EXPIRY_MS - 1).is_empty());
    assert_eq!(table.live(1_000 + EXPIRY_MS - 1), Some((grant.lease, grant.epoch)));
    assert_eq!(table.tick(1_000 + EXPIRY_MS), lost(&grant, LossReason::Expired));
    assert_eq!(table.live(1_000 + EXPIRY_MS), None);
    // Lost once: a later tick finds nothing to lose.
    assert!(table.tick(1_000 + 2 * EXPIRY_MS).is_empty());
}

#[test]
fn a_heartbeat_renews_within_the_expiry_and_a_late_one_finds_it_expired() {
    let mut table = LeaseTable::new(GENERATION);
    let (grant, _) = claim(&mut table, 0, 1);
    let renewed = table.request(EXPIRY_MS - 1, &heartbeat(&grant));
    assert_eq!(renewed.outcome, Ok(LeaseReply::Renewed { epoch: grant.epoch }));
    assert!(renewed.actions.is_empty());
    // Renewed from the heartbeat's moment, not the grant's.
    assert!(table.tick(2 * EXPIRY_MS - 2).is_empty());
    let late = table.request(2 * EXPIRY_MS - 1, &heartbeat(&grant));
    assert_eq!(late.outcome.expect_err("late").kind(), "daemon::lease_expired");
    assert_eq!(late.actions, lost(&grant, LossReason::Expired));
}

#[test]
fn a_second_claim_is_busy_until_the_first_is_lost() {
    let mut table = LeaseTable::new(GENERATION);
    let (first, _) = claim(&mut table, 0, 1);
    let rival = table.bind(5, StreamRole::Rpc, &LeaseIntent::Claim, fresh(2));
    assert_eq!(rival.outcome.expect_err("busy").kind(), "daemon::busy");
    assert!(rival.actions.is_empty());
    // Once the first expires, the claim made then is granted the next epoch.
    let later = table.bind(EXPIRY_MS, StreamRole::Rpc, &LeaseIntent::Claim, fresh(2));
    let Ok(LeaseBinding::Granted(second)) = later.outcome else { panic!("not granted") };
    assert_eq!(second.epoch, LeaseEpoch(2));
    let mut expected = lost(&first, LossReason::Expired);
    expected.push(LeaseAction::ResetChannel { epoch: LeaseEpoch(2) });
    assert_eq!(later.actions, expected);
}

#[test]
fn a_takeover_beats_a_claim_made_in_the_same_moment() {
    let mut table = LeaseTable::new(GENERATION);
    let (first, _) = claim(&mut table, 0, 1);
    let takeover = table.bind(0, StreamRole::Rpc, &LeaseIntent::Takeover, fresh(2));
    let Ok(LeaseBinding::Granted(second)) = takeover.outcome else { panic!("not granted") };
    assert_eq!(second.epoch, LeaseEpoch(2));
    let mut expected = lost(&first, LossReason::TakenOver);
    expected.push(LeaseAction::ResetChannel { epoch: LeaseEpoch(2) });
    assert_eq!(takeover.actions, expected);
    // The claim behind it is busy, and the old holder learns its lease was taken.
    assert_eq!(table.bind(0, StreamRole::Rpc, &LeaseIntent::Claim, fresh(3)).outcome.expect_err("busy").kind(), "daemon::busy");
    let old = table.request(1, &heartbeat(&first)).outcome.expect_err("taken");
    assert!(old.message().contains("taken over"), "{}", old.message());
}

#[test]
fn a_release_loses_the_lease_and_the_next_claim_starts_a_new_epoch() {
    let mut table = LeaseTable::new(GENERATION);
    let (grant, _) = claim(&mut table, 0, 1);
    let released = table.request(10, &LeaseRequest::Release { lease: grant.lease, token: grant.token });
    assert_eq!(released.outcome, Ok(LeaseReply::Released));
    assert_eq!(released.actions, lost(&grant, LossReason::Released));
    let after = table.request(11, &heartbeat(&grant)).outcome.expect_err("released");
    assert!(after.message().contains("released"), "{}", after.message());
    let (next, actions) = claim(&mut table, 12, 2);
    assert_eq!(next.epoch, LeaseEpoch(2));
    assert_eq!(actions, vec![LeaseAction::ResetChannel { epoch: LeaseEpoch(2) }]);
}

#[test]
fn a_resume_keeps_the_epoch_and_renews_and_an_attachment_binds_without_renewing() {
    let mut table = LeaseTable::new(GENERATION);
    let (grant, _) = claim(&mut table, 0, 1);
    let resumed = table.bind(20_000, StreamRole::Rpc, &resume(&grant), fresh(9));
    assert_eq!(resumed.outcome, Ok(LeaseBinding::Granted(grant.clone())));
    assert!(resumed.actions.is_empty(), "a resume resets nothing");
    assert!(table.tick(20_000 + EXPIRY_MS - 1).is_empty(), "renewed from the resume");
    let attached = table.bind(25_000, StreamRole::Events, &attach(&grant), fresh(9));
    assert_eq!(attached.outcome, Ok(LeaseBinding::Attached { lease: grant.lease, epoch: grant.epoch }));
    assert_eq!(table.tick(20_000 + EXPIRY_MS), lost(&grant, LossReason::Expired), "an attachment renews nothing");
}

#[test]
fn each_wrong_binding_is_refused() {
    let mut table = LeaseTable::new(GENERATION);
    let (grant, _) = claim(&mut table, 0, 1);
    let wrong = |intent: LeaseIntent| -> Vec<(&'static str, LeaseIntent)> {
        let LeaseIntent::Resume { lease, token, epoch, generation } = intent else { unreachable!() };
        vec![
            ("lease", LeaseIntent::Resume { lease: LeaseId([0xee; 16]), token, epoch, generation }),
            ("token", LeaseIntent::Resume { lease, token: LeaseToken([0xee; 32]), epoch, generation }),
            ("epoch", LeaseIntent::Resume { lease, token, epoch: LeaseEpoch(7), generation }),
            ("generation", LeaseIntent::Resume { lease, token, epoch, generation: WorkerGeneration([0xee; 16]) }),
        ]
    };
    for (name, intent) in wrong(resume(&grant)) {
        let change = table.bind(1, StreamRole::Rpc, &intent, fresh(9));
        assert_eq!(change.outcome.expect_err(name).kind(), "daemon::lease_invalid", "a resume with a wrong {name}");
        let LeaseIntent::Resume { lease, token, epoch, generation } = intent else { unreachable!() };
        let change = table.bind(1, StreamRole::Events, &LeaseIntent::Attach { lease, token, epoch, generation }, fresh(9));
        assert_eq!(change.outcome.expect_err(name).kind(), "daemon::lease_invalid", "an attachment with a wrong {name}");
    }
    let forged = LeaseRequest::Heartbeat { lease: grant.lease, token: LeaseToken([0xee; 32]) };
    assert_eq!(table.request(1, &forged).outcome.expect_err("forged").kind(), "daemon::lease_invalid");
    // None of them disturbed the live lease.
    assert_eq!(table.live(1), Some((grant.lease, grant.epoch)));
}

#[test]
fn a_stream_binds_only_the_intents_its_role_allows() {
    let grant = LeaseGrant { lease: LeaseId([1; 16]), token: LeaseToken([2; 32]), epoch: LeaseEpoch(1), heartbeat_ms: 10_000, expiry_ms: 30_000 };
    let intents = [LeaseIntent::Claim, resume(&grant), LeaseIntent::Takeover, attach(&grant), LeaseIntent::Observe];
    let allowed = |role: StreamRole, intent: &LeaseIntent| match role {
        StreamRole::Rpc => matches!(intent, LeaseIntent::Claim | LeaseIntent::Resume { .. } | LeaseIntent::Takeover),
        StreamRole::Events => matches!(intent, LeaseIntent::Attach { .. }),
        StreamRole::Status => matches!(intent, LeaseIntent::Observe),
    };
    for role in [StreamRole::Rpc, StreamRole::Events, StreamRole::Status] {
        for intent in &intents {
            assert_eq!(intent.fits(role), allowed(role, intent), "{role:?} {}", intent.name());
            if !allowed(role, intent) {
                let mut table = LeaseTable::new(GENERATION);
                let change = table.bind(0, role, intent, fresh(1));
                assert_eq!(change.outcome.expect_err("escalation").kind(), "daemon::invalid_frame", "{role:?} {}", intent.name());
                assert_eq!(table.live(0), None, "{role:?} {} bound a lease", intent.name());
            }
        }
    }
    // A Status stream observes and holds nothing.
    let mut table = LeaseTable::new(GENERATION);
    assert_eq!(table.bind(0, StreamRole::Status, &LeaseIntent::Observe, fresh(1)).outcome, Ok(LeaseBinding::Unbound));
    assert_eq!(table.live(0), None);
}

#[test]
fn a_token_never_reaches_debug_output() {
    let token = LeaseToken([0xab; 32]);
    assert_eq!(format!("{token:?}"), "LeaseToken(redacted)");
    let grant = LeaseGrant { lease: LeaseId([1; 16]), token, epoch: LeaseEpoch(1), heartbeat_ms: 10_000, expiry_ms: 30_000 };
    for shown in [
        format!("{:?}", resume(&grant)),
        format!("{grant:?}"),
        format!("{:?}", LeaseRequest::Release { lease: grant.lease, token }),
        format!("{:?}", Fresh { lease: grant.lease, token }),
    ] {
        assert!(!shown.contains("171"), "a token's byte in {shown}");
        assert!(shown.contains("redacted"), "{shown}");
    }
}

#[test]
fn no_loss_touches_an_operation() {
    let mut table = LeaseTable::new(GENERATION);
    let mut seen = Vec::new();
    let (first, actions) = claim(&mut table, 0, 1);
    seen.extend(actions);
    let takeover = table.bind(1, StreamRole::Rpc, &LeaseIntent::Takeover, fresh(2));
    seen.extend(takeover.actions);
    let Ok(LeaseBinding::Granted(second)) = takeover.outcome else { panic!("not granted") };
    seen.extend(table.request(2, &LeaseRequest::Release { lease: second.lease, token: second.token }).actions);
    let (_, actions) = claim(&mut table, 3, 3);
    seen.extend(actions);
    seen.extend(table.tick(3 + EXPIRY_MS));
    assert!(seen.iter().all(|action| ["events", "admission", "streams", "channel"].contains(&touches(action))));
    let notices = seen.iter().filter(|action| matches!(action, LeaseAction::Notify(_))).count();
    assert_eq!(notices, 3, "one notice a loss: taken over, released, expired");
    assert!(!seen.contains(&LeaseAction::StopAdmission { lease: LeaseId([9; 16]) }));
    assert_eq!(first.epoch, LeaseEpoch(1));
}

/// Every lease message variant's bitcode.
fn vectors() -> BTreeMap<String, golden::Vector> {
    let grant = LeaseGrant { lease: LeaseId([1; 16]), token: LeaseToken([2; 32]), epoch: LeaseEpoch(3), heartbeat_ms: 10_000, expiry_ms: 30_000 };
    let mut out = BTreeMap::new();
    let mut put = |name: &str, vector: golden::Vector| {
        out.insert(name.to_string(), vector);
    };
    put("intent/claim", golden::vector(&LeaseIntent::Claim));
    put("intent/resume", golden::vector(&resume(&grant)));
    put("intent/takeover", golden::vector(&LeaseIntent::Takeover));
    put("intent/attach", golden::vector(&attach(&grant)));
    put("intent/observe", golden::vector(&LeaseIntent::Observe));
    put("binding/granted", golden::vector(&LeaseBinding::Granted(grant.clone())));
    put("binding/attached", golden::vector(&LeaseBinding::Attached { lease: grant.lease, epoch: grant.epoch }));
    put("binding/unbound", golden::vector(&LeaseBinding::Unbound));
    put("request/heartbeat", golden::vector(&heartbeat(&grant)));
    put("request/release", golden::vector(&LeaseRequest::Release { lease: grant.lease, token: grant.token }));
    put("reply/renewed", golden::vector(&LeaseReply::Renewed { epoch: grant.epoch }));
    put("reply/released", golden::vector(&LeaseReply::Released));
    for (name, reason) in [("expired", LossReason::Expired), ("released", LossReason::Released), ("taken_over", LossReason::TakenOver)] {
        put(&format!("notice/lost/{name}"), golden::vector(&LeaseNotice::Lost { lease: grant.lease, epoch: grant.epoch, reason }));
    }
    out
}

#[test]
fn each_message_has_its_golden_vector_and_round_trips() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "a lease message's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let vectors = vectors();
    let intent: LeaseIntent = decode_payload(&vectors["intent/resume"].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    assert!(matches!(intent, LeaseIntent::Resume { epoch: LeaseEpoch(3), .. }));
    let binding: LeaseBinding = decode_payload(&vectors["binding/granted"].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    assert!(matches!(binding, LeaseBinding::Granted(LeaseGrant { epoch: LeaseEpoch(3), .. })));
    let notice: LeaseNotice = decode_payload(&vectors["notice/lost/taken_over"].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    assert_eq!(notice, LeaseNotice::Lost { lease: LeaseId([1; 16]), epoch: LeaseEpoch(3), reason: LossReason::TakenOver });
}
