//! An operation's contract: every state and event pair, one hash for every byte
//! form of a request, tickets at their expiry and past their use, foreign and
//! forged tickets, nothing executed but by an admission, a cancellation racing a
//! completion, a restart, retired results, each request's class, and golden
//! vectors.

use std::collections::BTreeMap;

use sourcetrait_common::datum::Nonce;
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/operation.nuon";

const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const EPOCH: LeaseEpoch = LeaseEpoch(3);
const EXPIRY_MS: u64 = 60_000;

fn op(n: u8) -> OperationId {
    OperationId([n; 16])
}

fn nonce(n: u64) -> Nonce {
    Nonce::new_unchecked(n)
}

/// An `execute` request carrying `nonce`.
fn an_execute(nonce: Nonce, data: &str) -> RuntimeRequest {
    RuntimeRequest::Engine(ToEngineSys::Execute(ExecuteRequest {
        host: Host::Local,
        nonce,
        definition: "def execute [data: nothing]: nothing -> nothing { null }".to_string(),
        data: data.to_string(),
        timeout: None,
    }))
}

/// `request` submitted under `ticket`, its hash its own.
fn submission(ticket: &OperationTicket, request: RuntimeRequest) -> Submission {
    Submission { correlation: 9, ticket: ticket.clone(), hash: payload_hash(&request), request }
}

/// A table with ticket `n` issued at 0 for its own nonce, and that ticket.
fn issued(n: u8) -> (OperationTable, OperationTicket) {
    let mut table = OperationTable::new(GENERATION);
    let ticket = table.issue(0, op(n), nonce(u64::from(n)), EPOCH).expect("issued");
    (table, ticket)
}

/// The admission of ticket `n`'s own request at `now_ms`.
fn admit(table: &mut OperationTable, ticket: &OperationTicket, now_ms: u64) -> (Admission, Option<Execute>) {
    table.submit(now_ms, &submission(ticket, an_execute(ticket.nonce, "null")), EPOCH)
}

fn refused_kind(admission: &Admission) -> &'static str {
    match &admission.outcome {
        AdmissionOutcome::Refused(error) => error.kind(),
        other => panic!("not refused: {other:?}"),
    }
}

const STATES: [OperationState; 7] = [
    OperationState::Admitted,
    OperationState::Running,
    OperationState::Succeeded,
    OperationState::Failed,
    OperationState::CancelRequested,
    OperationState::Cancelled,
    OperationState::UnknownAfterRestart,
];

const EVENTS: [OperationEvent; 6] = [
    OperationEvent::Dispatch,
    OperationEvent::Succeed,
    OperationEvent::Fail,
    OperationEvent::RequestCancel,
    OperationEvent::Cancelled,
    OperationEvent::Restart,
];

#[test]
fn every_state_and_event_pair_is_legal_or_refused() {
    use OperationEvent as E;
    use OperationState as S;
    let legal: BTreeMap<(&str, &str), S> = [
        ((S::Admitted, E::Dispatch), S::Running),
        ((S::Admitted, E::Fail), S::Failed),
        ((S::Admitted, E::RequestCancel), S::CancelRequested),
        ((S::Admitted, E::Restart), S::UnknownAfterRestart),
        ((S::Running, E::Succeed), S::Succeeded),
        ((S::Running, E::Fail), S::Failed),
        ((S::Running, E::RequestCancel), S::CancelRequested),
        ((S::Running, E::Restart), S::UnknownAfterRestart),
        ((S::CancelRequested, E::Succeed), S::Succeeded),
        ((S::CancelRequested, E::Fail), S::Failed),
        ((S::CancelRequested, E::RequestCancel), S::CancelRequested),
        ((S::CancelRequested, E::Cancelled), S::Cancelled),
        ((S::CancelRequested, E::Restart), S::UnknownAfterRestart),
    ]
    .into_iter()
    .map(|((state, event), to)| ((leak(format!("{state:?}")), leak(format!("{event:?}"))), to))
    .collect();
    for state in STATES {
        for event in EVENTS {
            let key = (leak(format!("{state:?}")), leak(format!("{event:?}")));
            assert_eq!(transition(state, event), legal.get(&key).copied(), "{state:?} on {event:?}");
            if state.terminal() {
                assert_eq!(transition(state, event), None, "a terminal {state:?} moved on {event:?}");
            }
        }
    }
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

#[test]
fn every_byte_form_of_a_request_shares_its_hash_and_a_changed_one_does_not() {
    let request = an_execute(nonce(7), "{a: 1}");
    let canonical = bitcode::encode(&request);
    let mut forms = 0;
    for at in 0..canonical.len() {
        for byte in 0..=255u8 {
            if byte == canonical[at] {
                continue;
            }
            let mut form = canonical.clone();
            form[at] = byte;
            if let Ok(decoded) = decode_payload::<RuntimeRequest>(&form, DECODE_BUDGET_DEFAULT) {
                if decoded == request {
                    forms += 1;
                    assert_eq!(payload_hash(&decoded), payload_hash(&request), "byte {at} as {byte:#04x}");
                } else {
                    assert_ne!(payload_hash(&decoded), payload_hash(&request), "byte {at} as {byte:#04x}");
                }
            }
        }
    }
    println!("{forms} other byte forms of {} bytes decode to the same request", canonical.len());
    assert_ne!(payload_hash(&an_execute(nonce(7), "{a: 2}")), payload_hash(&request));
}

#[test]
fn a_ticket_runs_until_its_expiry_and_only_once() {
    let (mut table, ticket) = issued(1);
    assert_eq!(ticket.expires_ms, 60_000);
    let (admission, execute) = admit(&mut table, &ticket, EXPIRY_MS - 1);
    assert_eq!(admission.outcome, AdmissionOutcome::Admitted);
    assert_eq!(execute, Some(Execute { operation: op(1), nonce: nonce(1) }));
    // The same submission again answers the admitted operation, running nothing.
    let (again, execute) = admit(&mut table, &ticket, EXPIRY_MS - 1);
    assert_eq!(again.outcome, AdmissionOutcome::Existing(OperationStatus::Known { operation: op(1), state: OperationState::Admitted }));
    assert_eq!(execute, None);
    // A ticket submitted at its expiry runs nothing.
    let late = table.issue(0, op(2), nonce(2), EPOCH).expect("issued");
    let (admission, execute) = admit(&mut table, &late, EXPIRY_MS);
    assert_eq!(refused_kind(&admission), "daemon::ticket_invalid");
    assert_eq!(execute, None);
}

#[test]
fn the_same_operation_with_another_payload_conflicts() {
    let (mut table, ticket) = issued(1);
    admit(&mut table, &ticket, 0);
    let changed = submission(&ticket, an_execute(ticket.nonce, "{changed: true}"));
    let (admission, execute) = table.submit(1, &changed, EPOCH);
    assert_eq!(refused_kind(&admission), "daemon::operation_conflict");
    assert_eq!(execute, None);
    assert_eq!(table.lookup(op(1)), OperationStatus::Known { operation: op(1), state: OperationState::Admitted });
}

#[test]
fn a_duplicate_claiming_the_first_hash_over_another_body_is_refused() {
    let (mut table, ticket) = issued(1);
    admit(&mut table, &ticket, 0);
    let first = submission(&ticket, an_execute(ticket.nonce, "null"));
    let forged = Submission { request: an_execute(ticket.nonce, "{changed: true}"), ..first.clone() };
    let refused = |table: &mut OperationTable, stage: &str| {
        let (admission, execute) = table.submit(1, &forged, EPOCH);
        assert_eq!(refused_kind(&admission), "daemon::invalid_frame", "{stage}");
        assert_eq!(execute, None, "{stage}");
        let (admission, execute) = table.submit(1, &first, EPOCH);
        assert!(matches!(admission.outcome, AdmissionOutcome::Existing(_)), "{stage}: the true duplicate");
        assert_eq!(execute, None, "{stage}");
    };
    refused(&mut table, "while live");
    assert_eq!(table.advance(op(1), OperationEvent::Dispatch), Some(OperationState::Running));
    let terminal = table.advance(op(1), OperationEvent::Succeed).expect("the operation completes");
    assert!(table.retire(op(1)), "the terminal record retires");
    refused(&mut table, "after its retirement");
    let mut recovered = OperationTable::recover(GENERATION, [(op(1), first.hash, terminal)]);
    refused(&mut recovered, "after a recovery");
}

#[test]
fn a_foreign_forged_or_unknown_ticket_runs_nothing() {
    let (mut table, ticket) = issued(1);
    let other_worker = OperationTable::new(WorkerGeneration([0x99; 16]));
    let cases = [
        ("an unknown operation", submission(&OperationTicket { operation: op(5), ..ticket.clone() }, an_execute(ticket.nonce, "null")), EPOCH, "daemon::ticket_invalid"),
        ("another generation", submission(&OperationTicket { generation: WorkerGeneration([0x99; 16]), ..ticket.clone() }, an_execute(ticket.nonce, "null")), EPOCH, "daemon::ticket_invalid"),
        ("another lease epoch", submission(&ticket, an_execute(ticket.nonce, "null")), LeaseEpoch(4), "daemon::ticket_invalid"),
        ("a forged nonce", submission(&OperationTicket { nonce: nonce(77), ..ticket.clone() }, an_execute(nonce(77), "null")), EPOCH, "daemon::ticket_invalid"),
        ("a request's nonce", submission(&ticket, an_execute(nonce(78), "null")), EPOCH, "daemon::ticket_invalid"),
        (
            "a wrong hash",
            Submission { hash: [0; 32], ..submission(&ticket, an_execute(ticket.nonce, "null")) },
            EPOCH,
            "daemon::invalid_frame",
        ),
    ];
    for (name, submission, epoch, kind) in cases {
        let (admission, execute) = table.submit(1, &submission, epoch);
        assert_eq!(refused_kind(&admission), kind, "{name}");
        assert_eq!(execute, None, "{name}");
    }
    // Every refusal left the ticket valid for its own submission.
    let (admission, execute) = admit(&mut table, &ticket, 2);
    assert_eq!(admission.outcome, AdmissionOutcome::Admitted);
    assert!(execute.is_some());
    drop(other_worker);
}

#[test]
fn nothing_but_an_admission_executes() {
    let (mut table, ticket) = issued(1);
    // Lookups of an issued, an unknown, and an admitted operation run nothing.
    assert_eq!(table.lookup(op(1)), OperationStatus::Unknown { operation: op(1) });
    assert_eq!(table.lookup(op(8)), OperationStatus::Unknown { operation: op(8) });
    let (_, execute) = admit(&mut table, &ticket, 0);
    assert!(execute.is_some(), "an admission executes");
    assert_eq!(table.lookup(op(1)), OperationStatus::Known { operation: op(1), state: OperationState::Admitted });
    // An unknown id, an expired ticket, and a repeat each execute nothing.
    let unknown = submission(&OperationTicket { operation: op(8), ..ticket.clone() }, an_execute(ticket.nonce, "null"));
    assert_eq!(table.submit(1, &unknown, EPOCH).1, None);
    let expired = table.issue(0, op(2), nonce(2), EPOCH).expect("issued");
    assert_eq!(admit(&mut table, &expired, EXPIRY_MS).1, None);
    assert_eq!(admit(&mut table, &ticket, 1).1, None);
}

#[test]
fn a_cancellation_racing_a_completion_ends_in_one_terminal() {
    for completion in [OperationEvent::Succeed, OperationEvent::Fail] {
        let (mut table, ticket) = issued(1);
        admit(&mut table, &ticket, 0);
        assert_eq!(table.advance(op(1), OperationEvent::Dispatch), Some(OperationState::Running));
        assert_eq!(table.advance(op(1), OperationEvent::RequestCancel), Some(OperationState::CancelRequested));
        let terminal = table.advance(op(1), completion).expect("the late completion lands");
        assert!(terminal.terminal());
        assert_eq!(table.advance(op(1), OperationEvent::Cancelled), None, "a second terminal");
        assert_eq!(table.lookup(op(1)), OperationStatus::Known { operation: op(1), state: terminal });
    }
    // Cancelled first, a completion after it lands nowhere.
    let (mut table, ticket) = issued(1);
    admit(&mut table, &ticket, 0);
    table.advance(op(1), OperationEvent::RequestCancel);
    assert_eq!(table.advance(op(1), OperationEvent::Cancelled), Some(OperationState::Cancelled));
    assert_eq!(table.advance(op(1), OperationEvent::Succeed), None);
    assert_eq!(table.lookup(op(1)), OperationStatus::Known { operation: op(1), state: OperationState::Cancelled });
}

#[test]
fn a_restart_leaves_every_unfinished_operation_unknown_and_runs_none_again() {
    let hash = payload_hash(&an_execute(nonce(1), "null"));
    let records = STATES.iter().enumerate().map(|(n, state)| (op(n as u8), hash, *state));
    let table = OperationTable::recover(WorkerGeneration([0x45; 16]), records);
    for (n, state) in STATES.iter().enumerate() {
        let expected = if state.terminal() { *state } else { OperationState::UnknownAfterRestart };
        assert_eq!(table.lookup(op(n as u8)), OperationStatus::Known { operation: op(n as u8), state: expected }, "{state:?}");
    }
    // A resubmission of a recovered operation answers it and runs nothing.
    let mut table = table;
    let ticket = OperationTicket { operation: op(1), nonce: nonce(1), generation: GENERATION, epoch: EPOCH, expires_ms: 60_000 };
    let request = an_execute(nonce(1), "null");
    let resubmitted = Submission { correlation: 1, ticket, hash, request };
    let (admission, execute) = table.submit(0, &resubmitted, EPOCH);
    assert!(matches!(admission.outcome, AdmissionOutcome::Existing(_)), "{admission:?}");
    assert_eq!(execute, None);
}

#[test]
fn a_retired_result_keeps_a_tombstone_its_id_never_runs_again() {
    let (mut table, ticket) = issued(1);
    admit(&mut table, &ticket, 0);
    assert!(!table.retire(op(1)), "an unfinished operation retired");
    table.advance(op(1), OperationEvent::Dispatch);
    table.advance(op(1), OperationEvent::Succeed);
    assert!(table.retire(op(1)));
    let expired = OperationStatus::Expired { operation: op(1), terminal: Some(OperationState::Succeeded) };
    assert_eq!(table.lookup(op(1)), expired);
    let (again, execute) = admit(&mut table, &ticket, 1);
    assert_eq!(again.outcome, AdmissionOutcome::Existing(expired));
    assert_eq!(execute, None);
    let changed = submission(&ticket, an_execute(ticket.nonce, "{changed: true}"));
    assert_eq!(refused_kind(&table.submit(1, &changed, EPOCH).0), "daemon::operation_conflict");
    assert_eq!(table.advance(op(1), OperationEvent::Fail), None, "a retired operation moved");
}

#[test]
fn tickets_awaiting_submission_are_bounded() {
    let mut table = OperationTable::new(GENERATION);
    for n in 0..TICKETS_MAX {
        let id = OperationId(u128::try_from(n).expect("fits").to_be_bytes());
        table.issue(0, id, nonce(n as u64), EPOCH).expect("within the bound");
    }
    let past = OperationId([0xff; 16]);
    assert_eq!(table.issue(1, past, nonce(1 << 40), EPOCH).expect_err("past the bound").kind(), "daemon::busy");
    // Once the waiting ones expire, issuing resumes.
    assert!(table.issue(EXPIRY_MS, past, nonce(1 << 40), EPOCH).is_ok());
}

#[test]
fn each_request_has_its_class() {
    let control = |command| RuntimeRequest::Engine(ToEngineSys::Control(ControlRequest { host: Host::Local, command }));
    let interface = |text: &str| RuntimeRequest::Interface { command: text.to_string() };
    let artifact = RuntimeRequest::Artifact;
    let read_log = ReadLog { operation: op(1), stream: LogStream::Stdout, offset: 0, max_bytes: LOG_READ_MAX };
    let reserve = ReserveUpload { purpose: ArtifactPurpose::EquipSource, display_name: "geo.tar".to_string(), size: 1, sha256: [3; 32] };
    let cases = [
        ("info", RuntimeRequest::Info, RequestClass::Query),
        ("a lookup", RuntimeRequest::OperationLookup { operation: op(1) }, RequestClass::Query),
        ("an execute", an_execute(nonce(1), "null"), RequestClass::Mutating),
        ("processes", control(EngineCommand::Processes), RequestClass::Query),
        ("health", control(EngineCommand::Health), RequestClass::Query),
        ("an equip check", control(EngineCommand::EquipCheck { source: "x".to_string() }), RequestClass::Query),
        ("a kill", control(EngineCommand::Kill { nonce: "x".to_string() }), RequestClass::Mutating),
        ("a teardown", control(EngineCommand::Teardown), RequestClass::Mutating),
        ("an install", control(EngineCommand::EquipInstall { source: "x".to_string() }), RequestClass::Mutating),
        ("interface info", interface("interface info"), RequestClass::Query),
        ("interface remote channel list", interface("interface remote channel list"), RequestClass::Query),
        ("interface process kill", interface("interface process kill abc"), RequestClass::Mutating),
        ("interface purview set", interface("interface purview set ['core']"), RequestClass::Mutating),
        ("an unknown command", interface("interface teleport home"), RequestClass::Mutating),
        ("an inspect", artifact(ArtifactRequest::Inspect(InspectArtifact { id: ArtifactId([1; 16]) })), RequestClass::Query),
        ("a describe", artifact(ArtifactRequest::Describe(DescribeArtifact { id: ArtifactId([1; 16]) })), RequestClass::Query),
        ("a log read", artifact(ArtifactRequest::ReadLog(read_log)), RequestClass::Query),
        ("a reserve", artifact(ArtifactRequest::Reserve(reserve)), RequestClass::Mutating),
        ("a publish", artifact(ArtifactRequest::Publish(PublishUpload { id: ArtifactId([1; 16]) })), RequestClass::Mutating),
        ("an abort", artifact(ArtifactRequest::Abort(AbortUpload { id: ArtifactId([1; 16]) })), RequestClass::Mutating),
        ("a cancellation", RuntimeRequest::Cancel { target: op(1) }, RequestClass::Mutating),
        ("a policy", RuntimeRequest::Policy(ControllerPolicy { deny_tools: Vec::new(), deny_commands: Vec::new() }), RequestClass::Mutating),
    ];
    for (name, request, class) in cases {
        assert_eq!(request_class(&request), class, "{name}");
    }
    // Every catalog command has a class, the read-only ones queries.
    for spec in COMMANDS.iter() {
        let class = request_class(&interface(&format!("interface {}", spec.words())));
        let read_only = matches!(
            spec.kind,
            CommandKind::Info | CommandKind::Help | CommandKind::ProcessList | CommandKind::PurviewList | CommandKind::RemoteList | CommandKind::RemoteGet | CommandKind::EquipCheck
        );
        assert_eq!(class == RequestClass::Query, read_only, "interface {}", spec.words());
    }
}

/// Every operation message variant's bitcode.
fn vectors() -> BTreeMap<String, golden::Vector> {
    let ticket = OperationTicket { operation: op(1), nonce: nonce(7), generation: GENERATION, epoch: EPOCH, expires_ms: 60_000 };
    let request = an_execute(nonce(7), "null");
    let mut out = BTreeMap::new();
    let mut put = |name: String, vector: golden::Vector| {
        out.insert(name, vector);
    };
    put("ticket".into(), golden::vector(&ticket));
    put("ticket_request".into(), golden::vector(&TicketRequest { correlation: 1 }));
    put("ticket_reply/issued".into(), golden::vector(&TicketReply { correlation: 1, outcome: Ok(ticket.clone()) }));
    put("ticket_reply/refused".into(), golden::vector(&TicketReply { correlation: 1, outcome: Err(TransportError::Busy) }));
    put("submission".into(), golden::vector(&Submission { correlation: 2, ticket: ticket.clone(), hash: payload_hash(&request), request }));
    for (name, outcome) in [
        ("admitted", AdmissionOutcome::Admitted),
        ("existing", AdmissionOutcome::Existing(OperationStatus::Known { operation: op(1), state: OperationState::Running })),
        ("refused", AdmissionOutcome::Refused(TransportError::OperationConflict { operation: op(1) })),
    ] {
        put(format!("admission/{name}"), golden::vector(&Admission { correlation: 2, operation: op(1), outcome }));
    }
    put("status_query".into(), golden::vector(&StatusQuery { correlation: 3, operation: op(1) }));
    for (name, status) in [
        ("known", OperationStatus::Known { operation: op(1), state: OperationState::CancelRequested }),
        ("unknown", OperationStatus::Unknown { operation: op(1) }),
        ("expired", OperationStatus::Expired { operation: op(1), terminal: Some(OperationState::Succeeded) }),
        ("expired_untold", OperationStatus::Expired { operation: op(1), terminal: None }),
    ] {
        put(format!("status_reply/{name}"), golden::vector(&StatusReply { correlation: 3, status }));
    }
    put("response/reply".into(), golden::vector(&Response { correlation: 4, operation: op(1), outcome: Ok(RuntimeReply::Interface { result: NuVal::Nothing }) }));
    put("response/error".into(), golden::vector(&Response { correlation: 4, operation: op(1), outcome: Err(TransportError::Busy) }));
    for state in STATES {
        put(format!("state/{state:?}"), golden::vector(&state));
    }
    out
}

#[test]
fn each_message_has_its_golden_vector_and_round_trips() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an operation message's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let vectors = vectors();
    let ticket: OperationTicket = decode_payload(&vectors["ticket"].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    assert_eq!(ticket.operation, op(1));
    let submission: Submission = decode_payload(&vectors["submission"].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    assert_eq!(submission.hash, payload_hash(&submission.request));
    for state in STATES {
        let back: OperationState = decode_payload(&vectors[&format!("state/{state:?}")].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
        assert_eq!(back, state);
    }
}
