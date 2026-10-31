//! The operation journal: each move on disk before its answer, outcomes kept for
//! duplicates, room and faults refusing before anything is acknowledged, one
//! admission under concurrent duplicates, and its files private and locked.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nuval::NuVal;
use sourcetrait_common::datum::Nonce;
use sourcetrait_common::subsys;
use sourcetrait_grammar_journal::guts::{Fault, Faults, Journal, JournalSpec};
use sourcetrait_grammar_journal::*;
use sourcetrait_grammar_protocol::{
    AdmissionOutcome, ExecuteRequest, Host, Instance, LeaseEpoch, OperationId, OperationState, OperationStatus,
    OperationTicket, RuntimeReply, RuntimeRequest, Submission, ToEngineSys, TransportError, WorkerGeneration,
    payload_hash,
};

const GENERATION: WorkerGeneration = WorkerGeneration([0x51; 16]);
const EPOCH: LeaseEpoch = LeaseEpoch(2);
/// A reservation any test filesystem holds.
const RESERVE: u64 = 64 * 1024;

fn op(n: u8) -> OperationId {
    OperationId([n; 16])
}

fn spec(root: &Path, cap: u64) -> JournalSpec {
    JournalSpec { root: root.join("operations"), generation: GENERATION, instance: Instance::new("fixture", "default").unwrap(), cap }
}

fn open(root: &Path, faults: Faults) -> Journal {
    Journal::open(spec(root, CAP_DEFAULT), faults).expect("the journal opens")
}

/// A draw that gives operation `n` and nonce `n`.
fn ids(n: u8) -> impl FnMut() -> io::Result<(OperationId, Nonce)> {
    move || Ok((op(n), Nonce::new_unchecked(u64::from(n))))
}

/// An `execute` under `nonce` with `data`.
fn an_execute(nonce: Nonce, data: &str) -> RuntimeRequest {
    RuntimeRequest::Engine(ToEngineSys::Execute(ExecuteRequest {
        host: Host::Local,
        nonce,
        definition: "def execute [data: nothing]: nothing -> nothing { null }".to_string(),
        data: data.to_string(),
        timeout: None,
    }))
}

/// `request` under `ticket`, its hash its own.
fn submission(ticket: &OperationTicket, request: RuntimeRequest) -> Submission {
    Submission { correlation: 3, ticket: ticket.clone(), hash: payload_hash(&request), request }
}

/// Ticket `n` issued and its own request's submission.
fn ticketed(journal: &mut Journal, n: u8) -> Submission {
    let ticket = journal.issue(0, EPOCH, &mut ids(n)).expect("a ticket");
    submission(&ticket, an_execute(ticket.nonce, "null"))
}

/// An outcome carrying `text`.
fn outcome(text: &str) -> Result<RuntimeReply, TransportError> {
    Ok(RuntimeReply::Interface { result: NuVal::String(text.to_string()) })
}

/// Each line of the journal's segment, read as NUON.
fn lines(journal: &Journal) -> Vec<NuVal> {
    std::fs::read_to_string(journal.segment())
        .expect("the segment")
        .lines()
        .map(|line| NuVal::from_nuon(line).expect("a NUON line"))
        .collect()
}

/// A line's entry kind and its record.
fn entry(line: &NuVal) -> (String, NuVal) {
    let NuVal::Record(fields) = &line["entry"] else { panic!("an entry: {line:?}") };
    let (kind, record) = fields.first().expect("an entry's kind").clone();
    (kind, record)
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777
}

fn refused_kind(submitted: &Submitted) -> &'static str {
    match &submitted.admission.outcome {
        AdmissionOutcome::Refused(error) => error.kind(),
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn a_journal_is_private_locked_and_its_segment_its_own_start() {
    let home = tempfile::tempdir().unwrap();
    let journal = open(home.path(), Faults::none());
    let root = journal.root().to_path_buf();
    for dir in [root.clone(), root.join("segments"), root.join("results")] {
        assert_eq!(mode(&dir), 0o700, "{}", dir.display());
    }
    assert_eq!(journal.segment(), root.join("segments").join("1.nuonl"));
    assert_eq!(mode(journal.segment()), 0o600);
    let header = lines(&journal);
    assert_eq!(header.len(), 1, "the header alone");
    let (kind, opened) = entry(&header[0]);
    assert_eq!(kind, "Opened");
    assert_eq!(opened["generation"].as_str(), Some(GENERATION.hex().as_str()));
    assert_eq!(opened["instance"].as_str(), Some("fixture/default"));
    let refused = Journal::open(spec(home.path(), CAP_DEFAULT), Faults::none()).err().expect("a second journal");
    assert_eq!(refused.kind(), JournalErrorKind::Held, "{refused}");
    let first = std::fs::read(journal.segment()).unwrap();
    let first_path = journal.segment().to_path_buf();
    drop(journal);
    let restarted = open(home.path(), Faults::none());
    assert_eq!(restarted.segment(), root.join("segments").join("2.nuonl"), "a segment of its own");
    assert_eq!(std::fs::read(&first_path).unwrap(), first, "the first left as it was");
}

#[test]
fn a_root_not_private_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let open_to_others = home.path().join("operations");
    std::fs::create_dir(&open_to_others).unwrap();
    std::fs::set_permissions(&open_to_others, std::fs::Permissions::from_mode(0o755)).unwrap();
    let refused = Journal::open(spec(home.path(), CAP_DEFAULT), Faults::none()).err().expect("refused");
    assert_eq!(refused.kind(), JournalErrorKind::Unavailable, "{refused}");
    std::fs::remove_dir(&open_to_others).unwrap();
    let elsewhere = home.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::fs::set_permissions(&elsewhere, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &open_to_others).unwrap();
    let refused = Journal::open(spec(home.path(), CAP_DEFAULT), Faults::none()).err().expect("refused");
    assert_eq!(refused.kind(), JournalErrorKind::Unavailable, "a symlink: {refused}");
}

#[test]
fn each_move_is_on_disk_before_its_answer() {
    let home = tempfile::tempdir().unwrap();
    let mut journal = open(home.path(), Faults::none());
    let submitted = ticketed(&mut journal, 1);
    let admitted = journal.submit(1, &submitted, EPOCH, RESERVE);
    assert_eq!(admitted.admission.outcome, AdmissionOutcome::Admitted);
    let grant = admitted.grant.expect("a run");
    assert_eq!((grant.operation, grant.nonce), (op(1), submitted.ticket.nonce));
    let (kind, record) = entry(lines(&journal).last().unwrap());
    assert_eq!(kind, "Admitted");
    assert_eq!(record["operation"].as_str(), Some(op(1).hex().as_str()));
    assert_eq!(record["nonce"].as_str(), Some(submitted.ticket.nonce.to_string().as_str()));
    let hash: String = submitted.hash.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(record["hash"].as_str(), Some(hash.as_str()));
    assert_eq!(journal.advance(op(1), Step::Start), Ok(OperationState::Running));
    assert_eq!(entry(lines(&journal).last().unwrap()).0, "Running");
    assert_eq!(journal.finish(op(1), Ending::Succeed, &outcome("done")), Ok(OperationState::Succeeded));
    let (kind, finished) = entry(lines(&journal).last().unwrap());
    assert_eq!(kind, "Finished");
    assert_eq!(finished["state"].as_str(), Some("Succeeded"));
    assert!(matches!(&finished["outcome"]["Inline"], NuVal::Binary(_)), "{finished:?}");
    assert_eq!(journal.lookup(op(1)), OperationStatus::Known { operation: op(1), state: OperationState::Succeeded });
    assert_eq!(lines(&journal).len(), 4, "a header and three moves");
    let unknown = journal.advance(op(9), Step::Start).expect_err("no such operation");
    assert_eq!(unknown, StepError::Refused { operation: op(9), state: None });
    let ended = journal.advance(op(1), Step::Start).expect_err("a terminal moves no more");
    assert_eq!(ended, StepError::Refused { operation: op(1), state: Some(OperationState::Succeeded) });
}

#[test]
fn a_duplicate_of_a_finished_operation_gets_its_kept_outcome() {
    let home = tempfile::tempdir().unwrap();
    let mut journal = open(home.path(), Faults::none());
    let large = "x".repeat(INLINE_MAX + 1);
    for (n, text) in [(1, "small"), (2, large.as_str())] {
        let submitted = ticketed(&mut journal, n);
        journal.submit(1, &submitted, EPOCH, RESERVE);
        journal.advance(op(n), Step::Start).unwrap();
        journal.finish(op(n), Ending::Succeed, &outcome(text)).unwrap();
        let again = Submission { correlation: 8, ..submitted.clone() };
        let duplicate = journal.submit(2, &again, EPOCH, RESERVE);
        assert_eq!(
            duplicate.admission.outcome,
            AdmissionOutcome::Existing(OperationStatus::Known { operation: op(n), state: OperationState::Succeeded }),
        );
        assert!(duplicate.grant.is_none(), "a duplicate runs nothing");
        let response = duplicate.response.expect("its kept outcome");
        assert_eq!((response.correlation, response.operation), (8, op(n)));
        assert_eq!(response.outcome, outcome(text), "operation {n}'s outcome");
    }
    let (_, finished) = entry(lines(&journal).last().unwrap());
    let sha256 = finished["outcome"]["Sidecar"]["sha256"].as_str().expect("a sidecar").to_string();
    let sidecar = journal.root().join("results").join(format!("{sha256}.nuon"));
    assert_eq!(mode(&sidecar), 0o600, "{}", sidecar.display());
}

#[test]
fn a_conflicting_payload_and_a_forged_hash_run_nothing() {
    let home = tempfile::tempdir().unwrap();
    let mut journal = open(home.path(), Faults::none());
    let submitted = ticketed(&mut journal, 1);
    journal.submit(1, &submitted, EPOCH, RESERVE);
    let changed = submission(&submitted.ticket, an_execute(submitted.ticket.nonce, "{changed: true}"));
    let conflict = journal.submit(2, &changed, EPOCH, RESERVE);
    assert_eq!(refused_kind(&conflict), "daemon::operation_conflict");
    assert!(conflict.grant.is_none());
    let forged = Submission { hash: [0; 32], ..ticketed(&mut journal, 2) };
    assert_eq!(refused_kind(&journal.submit(2, &forged, EPOCH, RESERVE)), "daemon::invalid_frame");
    assert_eq!(journal.lookup(op(2)), OperationStatus::Unknown { operation: op(2) });
    assert_eq!(lines(&journal).len(), 2, "the one admission alone written");
}

#[test]
fn no_room_refuses_before_anything_is_written_or_run() {
    let home = tempfile::tempdir().unwrap();
    let mut journal = Journal::open(spec(home.path(), 1), Faults::none()).expect("the journal opens");
    let submitted = ticketed(&mut journal, 1);
    let before = std::fs::read(journal.segment()).unwrap();
    let full = journal.submit(1, &submitted, EPOCH, RESERVE);
    assert_eq!(refused_kind(&full), "daemon::storage_full");
    assert!(full.grant.is_none(), "nothing runs");
    assert_eq!(std::fs::read(journal.segment()).unwrap(), before, "nothing written");
    assert_eq!(journal.lookup(op(1)), OperationStatus::Unknown { operation: op(1) });
    drop(journal);
    let mut roomy = open(home.path(), Faults::none());
    let submitted = ticketed(&mut roomy, 2);
    let before = std::fs::read(roomy.segment()).unwrap();
    let past_any_disk = roomy.submit(1, &submitted, EPOCH, u64::MAX / 2);
    assert_eq!(refused_kind(&past_any_disk), "daemon::storage_full", "the free space");
    assert!(past_any_disk.grant.is_none());
    assert_eq!(std::fs::read(roomy.segment()).unwrap(), before);
    assert_eq!(roomy.submit(2, &submitted, EPOCH, RESERVE).admission.outcome, AdmissionOutcome::Admitted, "room for its own");
}

/// A journal with `fault` armed at its `nth` hit.
fn armed(home: &Path, fault: Fault, nth: u32) -> Journal {
    open(home, Faults::none().arm(fault, nth))
}

/// The degraded journal admits nothing and issues nothing more.
fn assert_degraded(journal: &mut Journal) {
    assert!(journal.degraded().is_some(), "degraded");
    let issued = journal.issue(5, EPOCH, &mut ids(7)).expect_err("no ticket from a degraded journal");
    assert_eq!(issued.kind(), "daemon::unavailable");
}

#[test]
fn a_failed_admission_write_acknowledges_nothing() {
    for fault in [Fault::LineWrite, Fault::LineSync] {
        let home = tempfile::tempdir().unwrap();
        let mut journal = armed(home.path(), fault, 2);
        let submitted = ticketed(&mut journal, 1);
        let refused = journal.submit(1, &submitted, EPOCH, RESERVE);
        assert_eq!(refused_kind(&refused), "daemon::unavailable", "{fault:?}");
        assert!(refused.grant.is_none(), "{fault:?}: nothing runs");
        assert_eq!(journal.lookup(op(1)), OperationStatus::Unknown { operation: op(1) }, "{fault:?}");
        if fault == Fault::LineWrite {
            assert_eq!(lines(&journal).len(), 1, "nothing past the header written");
        }
        assert_degraded(&mut journal);
    }
}

#[test]
fn a_failed_start_write_leaves_the_operation_admitted_and_unrun() {
    for fault in [Fault::LineWrite, Fault::LineSync] {
        let home = tempfile::tempdir().unwrap();
        let mut journal = armed(home.path(), fault, 3);
        let submitted = ticketed(&mut journal, 1);
        assert!(journal.submit(1, &submitted, EPOCH, RESERVE).grant.is_some());
        assert!(matches!(journal.advance(op(1), Step::Start), Err(StepError::Degraded { .. })), "{fault:?}");
        assert_eq!(journal.lookup(op(1)), OperationStatus::Known { operation: op(1), state: OperationState::Admitted });
        assert!(matches!(journal.finish(op(1), Ending::Fail, &outcome("x")), Err(StepError::Degraded { .. })));
        assert_degraded(&mut journal);
    }
}

#[test]
fn a_failed_finish_write_leaves_the_operation_running_with_no_outcome() {
    let large = "y".repeat(INLINE_MAX + 1);
    let cases = [
        (Fault::LineWrite, 4, "small"),
        (Fault::LineSync, 4, "small"),
        (Fault::SidecarCreate, 1, large.as_str()),
        (Fault::SidecarWrite, 1, large.as_str()),
        (Fault::SidecarSync, 1, large.as_str()),
        (Fault::SidecarRename, 1, large.as_str()),
        (Fault::SidecarDirSync, 1, large.as_str()),
        (Fault::LineWrite, 4, large.as_str()),
    ];
    for (fault, nth, text) in cases {
        let home = tempfile::tempdir().unwrap();
        let mut journal = armed(home.path(), fault, nth);
        let submitted = ticketed(&mut journal, 1);
        journal.submit(1, &submitted, EPOCH, RESERVE);
        journal.advance(op(1), Step::Start).unwrap();
        let failed = journal.finish(op(1), Ending::Succeed, &outcome(text));
        assert!(matches!(failed, Err(StepError::Degraded { .. })), "{fault:?}: {failed:?}");
        assert_eq!(journal.lookup(op(1)), OperationStatus::Known { operation: op(1), state: OperationState::Running }, "{fault:?}");
        let again = journal.submit(2, &Submission { correlation: 9, ..submitted.clone() }, EPOCH, RESERVE);
        assert!(again.grant.is_none() && again.response.is_none(), "{fault:?}: no outcome answered");
        if fault != Fault::LineSync {
            assert!(lines(&journal).iter().all(|line| entry(line).0 != "Finished"), "{fault:?}: no finish written");
        }
        assert_degraded(&mut journal);
    }
}

/// A handler that takes nothing, the journal sending nothing up.
struct Quiet;

impl subsys::Handler<JournalSystem> for Quiet {
    fn on_packet(
        &mut self,
        _pkt: subsys::Packet<FromJournalSys>,
    ) -> impl Future<Output = Option<subsys::Packet<FromJournalSys>>> + 'static + Send {
        std::future::ready(None)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_duplicates_through_the_system_admit_once() {
    let home = tempfile::tempdir().unwrap();
    let params = JournalSysParams { generation: GENERATION, instance: Instance::new("fixture", "default").unwrap(), cap: CAP_DEFAULT };
    let root: PathBuf = home.path().join("operations");
    let journal = Arc::new(JournalHandle::start(root.clone(), params.clone(), Quiet).await.expect("the system starts"));
    let held = JournalHandle::start(root.clone(), params, Quiet).await.expect_err("a second system");
    assert_eq!(held.kind(), JournalErrorKind::Held, "{held}");
    let ticket = journal.request(IssueRequest { epoch: EPOCH }).await.unwrap().expect("a ticket");
    let racers: Vec<_> = (0..24u8)
        .map(|n| {
            let journal = journal.clone();
            let data = if n % 3 == 0 { format!("{{n: {n}}}") } else { "null".to_string() };
            let submission = submission(&ticket, an_execute(ticket.nonce, &data));
            tokio::spawn(async move {
                let hash = submission.hash;
                let submitted = journal.request(SubmitRequest { submission: Box::new(submission), epoch: EPOCH, reserve: RESERVE }).await.unwrap();
                (hash, submitted)
            })
        })
        .collect();
    let mut answers = Vec::new();
    for racer in racers {
        answers.push(racer.await.unwrap());
    }
    let granted: Vec<_> = answers.iter().filter(|(_, submitted)| submitted.grant.is_some()).collect();
    assert_eq!(granted.len(), 1, "one admission runs");
    let winner = granted[0].0;
    for (hash, submitted) in &answers {
        match &submitted.admission.outcome {
            AdmissionOutcome::Admitted => assert_eq!(*hash, winner),
            AdmissionOutcome::Existing(_) => assert_eq!(*hash, winner, "a duplicate of the admitted payload"),
            AdmissionOutcome::Refused(TransportError::OperationConflict { .. }) => assert_ne!(*hash, winner),
            other => panic!("an answer that isn't the admission's: {other:?}"),
        }
    }
    let status = journal.request(LookupRequest { operation: ticket.operation }).await.unwrap();
    assert_eq!(status, OperationStatus::Known { operation: ticket.operation, state: OperationState::Admitted });
    Arc::into_inner(journal).expect("the last handle").stop().await.expect("stopped");
    JournalHandle::start(root, JournalSysParams { generation: GENERATION, instance: Instance::new("fixture", "default").unwrap(), cap: CAP_DEFAULT }, Quiet)
        .await
        .expect("the stop let its lock go")
        .stop()
        .await
        .unwrap();
}
