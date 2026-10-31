//! The stream roles' frames: every variant of each role's frames both ways,
//! its golden vector and its round trip, and each within its stream's frame.

use std::collections::{BTreeMap, BTreeSet};

use sourcetrait_common::datum::Nonce;
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/stream.nuon";

const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const OPERATION: OperationId = OperationId([0x11; 16]);

fn ticket() -> OperationTicket {
    OperationTicket { operation: OPERATION, nonce: Nonce::new_unchecked(7), generation: GENERATION, epoch: LeaseEpoch(1), expires_ms: 60_000 }
}

fn orderly() -> StreamClose {
    StreamClose { error: None }
}

fn rpc_frames() -> Vec<(&'static str, RpcFrame)> {
    let request = RuntimeRequest::Info;
    let submission = Submission { correlation: 2, ticket: ticket(), hash: payload_hash(&request), request };
    vec![
        ("ticket", RpcFrame::Ticket(TicketRequest { correlation: 1 })),
        ("submit", RpcFrame::Submit(Box::new(submission))),
        ("status", RpcFrame::Status(StatusQuery { correlation: 3, operation: OPERATION })),
        ("lease", RpcFrame::Lease(LeaseRequest::Heartbeat { lease: LeaseId([1; 16]), token: LeaseToken([2; 32]) })),
        ("ping", RpcFrame::Ping(Ping { nonce: 9 })),
        ("close", RpcFrame::Close(orderly())),
    ]
}

fn rpc_answers() -> Vec<(&'static str, RpcAnswer)> {
    let response = Response { correlation: 2, operation: OPERATION, outcome: Ok(RuntimeReply::Interface { result: NuVal::Nothing }) };
    let running = OperationStatus::Known { operation: OPERATION, state: OperationState::Running };
    vec![
        ("ticket", RpcAnswer::Ticket(TicketReply { correlation: 1, outcome: Ok(ticket()) })),
        ("admission", RpcAnswer::Admission(Admission { correlation: 2, operation: OPERATION, outcome: AdmissionOutcome::Admitted })),
        ("response", RpcAnswer::Response(Box::new(response))),
        ("status", RpcAnswer::Status(StatusReply { correlation: 3, status: running })),
        ("lease/renewed", RpcAnswer::Lease(Ok(LeaseReply::Renewed { epoch: LeaseEpoch(1) }))),
        ("lease/expired", RpcAnswer::Lease(Err(TransportError::LeaseExpired))),
        ("pong", RpcAnswer::Pong(Ping { nonce: 9 })),
        ("close", RpcAnswer::Close(StreamClose { error: Some(TransportError::LeaseExpired) })),
    ]
}

fn events_frames() -> Vec<(&'static str, EventsFrame)> {
    let cursor = EventCursor { generation: GENERATION, sequence: 7 };
    vec![
        ("resume", EventsFrame::Resume(EventResume { cursor: Some(cursor) })),
        ("ack", EventsFrame::Ack(EventAck { cursor })),
        ("ping", EventsFrame::Ping(Ping { nonce: 9 })),
        ("close", EventsFrame::Close(orderly())),
    ]
}

fn events_answers() -> Vec<(&'static str, EventsAnswer)> {
    let behind = TransportError::Unavailable { reason: "the subscriber fell past its window".to_string() };
    vec![
        ("event", EventsAnswer::Event(EventFrame::Gap { from: 2, to: 5 })),
        ("pong", EventsAnswer::Pong(Ping { nonce: 9 })),
        ("close", EventsAnswer::Close(StreamClose { error: Some(behind) })),
    ]
}

fn status_frames() -> Vec<(&'static str, StatusFrame)> {
    vec![
        ("lookup", StatusFrame::Lookup(StatusQuery { correlation: 1, operation: OPERATION })),
        ("health", StatusFrame::Health { correlation: 2 }),
        ("ping", StatusFrame::Ping(Ping { nonce: 9 })),
        ("close", StatusFrame::Close(orderly())),
    ]
}

fn status_answers() -> Vec<(&'static str, StatusAnswer)> {
    vec![
        ("lookup", StatusAnswer::Lookup(StatusReply { correlation: 1, status: OperationStatus::Unknown { operation: OPERATION } })),
        ("health", StatusAnswer::Health { correlation: 2, observation: Observation { engine: None, listening: true } }),
        ("pong", StatusAnswer::Pong(Ping { nonce: 9 })),
        ("close", StatusAnswer::Close(orderly())),
    ]
}

#[test]
fn each_role_carries_every_family_and_a_ping_and_a_close_both_ways() {
    let rpc: BTreeSet<_> = rpc_frames()
        .iter()
        .map(|(_, f)| match f {
            RpcFrame::Ticket(_) => "ticket",
            RpcFrame::Submit(_) => "submit",
            RpcFrame::Status(_) => "status",
            RpcFrame::Lease(_) => "lease",
            RpcFrame::Ping(_) => "ping",
            RpcFrame::Close(_) => "close",
        })
        .collect();
    assert_eq!(rpc.len(), 6);
    let rpc: BTreeSet<_> = rpc_answers()
        .iter()
        .map(|(_, a)| match a {
            RpcAnswer::Ticket(_) => "ticket",
            RpcAnswer::Admission(_) => "admission",
            RpcAnswer::Response(_) => "response",
            RpcAnswer::Status(_) => "status",
            RpcAnswer::Lease(_) => "lease",
            RpcAnswer::Pong(_) => "pong",
            RpcAnswer::Close(_) => "close",
        })
        .collect();
    assert_eq!(rpc.len(), 7);
    let events: BTreeSet<_> = events_frames()
        .iter()
        .map(|(_, f)| match f {
            EventsFrame::Resume(_) => "resume",
            EventsFrame::Ack(_) => "ack",
            EventsFrame::Ping(_) => "ping",
            EventsFrame::Close(_) => "close",
        })
        .collect();
    assert_eq!(events.len(), 4);
    let events: BTreeSet<_> = events_answers()
        .iter()
        .map(|(_, a)| match a {
            EventsAnswer::Event(_) => "event",
            EventsAnswer::Pong(_) => "pong",
            EventsAnswer::Close(_) => "close",
        })
        .collect();
    assert_eq!(events.len(), 3);
    let status: BTreeSet<_> = status_frames()
        .iter()
        .map(|(_, f)| match f {
            StatusFrame::Lookup(_) => "lookup",
            StatusFrame::Health { .. } => "health",
            StatusFrame::Ping(_) => "ping",
            StatusFrame::Close(_) => "close",
        })
        .collect();
    assert_eq!(status.len(), 4);
    let status: BTreeSet<_> = status_answers()
        .iter()
        .map(|(_, a)| match a {
            StatusAnswer::Lookup(_) => "lookup",
            StatusAnswer::Health { .. } => "health",
            StatusAnswer::Pong(_) => "pong",
            StatusAnswer::Close(_) => "close",
        })
        .collect();
    assert_eq!(status.len(), 4);
}

/// Every stream frame's bitcode, with its stream's ceiling.
fn vectors() -> (BTreeMap<String, golden::Vector>, BTreeMap<String, u32>) {
    let mut out = BTreeMap::new();
    let mut ceilings = BTreeMap::new();
    let rpc = StreamRole::Rpc.default_ceiling();
    let events = StreamRole::Events.default_ceiling();
    let status = StreamRole::Status.default_ceiling();
    let mut put = |name: String, vector: golden::Vector, ceiling: u32| {
        ceilings.insert(name.clone(), ceiling);
        out.insert(name, vector);
    };
    for (name, frame) in rpc_frames() {
        put(format!("rpc/frame/{name}"), golden::vector(&frame), rpc);
    }
    for (name, answer) in rpc_answers() {
        put(format!("rpc/answer/{name}"), golden::vector(&answer), rpc);
    }
    for (name, frame) in events_frames() {
        put(format!("events/frame/{name}"), golden::vector(&frame), events);
    }
    for (name, answer) in events_answers() {
        put(format!("events/answer/{name}"), golden::vector(&answer), events);
    }
    for (name, frame) in status_frames() {
        put(format!("status/frame/{name}"), golden::vector(&frame), status);
    }
    for (name, answer) in status_answers() {
        put(format!("status/answer/{name}"), golden::vector(&answer), status);
    }
    (out, ceilings)
}

/// A value's bitcode round trip within the default budget, compared whole.
fn round_trips<T>(name: &str, value: &T)
where
    T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
{
    let back: T = decode_payload(&bitcode::encode(value), DECODE_BUDGET_DEFAULT).expect(name);
    assert_eq!(&back, value, "{name}");
}

#[test]
fn each_frame_has_its_golden_vector_round_trips_and_fits_its_stream() {
    let (vectors, ceilings) = vectors();
    for (name, vector) in &vectors {
        assert!(vector.bytes.len() <= ceilings[name] as usize, "{name} fits its stream's frame");
    }
    let actual = golden::hex_table(vectors);
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "a stream frame's bytes changed; a deliberate change re-blesses {GOLDEN}");
    rpc_frames().iter().for_each(|(name, f)| round_trips(name, f));
    rpc_answers().iter().for_each(|(name, a)| round_trips(name, a));
    events_frames().iter().for_each(|(name, f)| round_trips(name, f));
    events_answers().iter().for_each(|(name, a)| round_trips(name, a));
    status_frames().iter().for_each(|(name, f)| round_trips(name, f));
    status_answers().iter().for_each(|(name, a)| round_trips(name, a));
}
