//! The event stream: a contiguous resume, stale and future cursors, a lost
//! range, a restart's reset, acks moving receipt alone, the exact records and
//! bytes ceilings, an epoch changed with events pending, the Channel gate's
//! order, a slow subscriber gapped and then closed, and golden vectors.

use std::collections::BTreeMap;

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/event.nuon";

const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const RESTARTED: WorkerGeneration = WorkerGeneration([0x45; 16]);

fn host(n: i64) -> EventPayload {
    EventPayload::Host(HostEvent {
        id: format!("event{n}"),
        from: "thread/fixture".to_string(),
        model: "fixture/thing/Changed".to_string(),
        event: NuVal::Int(n),
        attached: None,
    })
}

/// A backlog held to `records` records and plenty of bytes.
fn backlog(records: u32) -> EventBacklog {
    EventBacklog::new(GENERATION, &Limits { event_records: records, ..LIMITS_DEFAULT })
}

/// `n` events pushed, numbered from 1.
fn filled(records: u32, n: i64) -> EventBacklog {
    let mut backlog = backlog(records);
    for i in 1..=n {
        backlog.push(None, host(i), Vec::new()).expect("pushed");
    }
    backlog
}

fn at(sequence: u64) -> Option<EventCursor> {
    Some(EventCursor { generation: GENERATION, sequence })
}

/// The sequences of the records among `frames`.
fn sequences(frames: &[EventFrame]) -> Vec<u64> {
    frames
        .iter()
        .filter_map(|frame| match frame {
            EventFrame::Record(record) => Some(record.sequence),
            _ => None,
        })
        .collect()
}

#[test]
fn a_contiguous_resume_reads_from_the_next_sequence() {
    let backlog = filled(16, 3);
    assert_eq!((backlog.oldest(), backlog.newest()), (1, Some(3)));
    assert_eq!(sequences(&backlog.read(None, 100).expect("read")), vec![1, 2, 3]);
    assert_eq!(sequences(&backlog.read(at(1), 100).expect("read")), vec![2, 3]);
    assert!(backlog.read(at(3), 100).expect("read").is_empty(), "an up-to-date cursor reads nothing");
    assert_eq!(sequences(&backlog.read(at(0), 2).expect("read")), vec![1, 2], "at most max records");
}

#[test]
fn a_stale_cursor_reads_the_lost_range_as_a_gap_then_the_oldest() {
    let backlog = filled(4, 6);
    assert_eq!(backlog.oldest(), 3);
    let frames = backlog.read(at(1), 100).expect("read");
    assert_eq!(frames.first(), Some(&EventFrame::Gap { from: 2, to: 2 }));
    assert_eq!(sequences(&frames), vec![3, 4, 5, 6]);
    let frames = backlog.read(at(0), 100).expect("read");
    assert_eq!(frames.first(), Some(&EventFrame::Gap { from: 1, to: 2 }));
    assert_eq!(sequences(&backlog.read(at(2), 100).expect("read")), vec![3, 4, 5, 6], "no gap when nothing it wanted was lost");
}

#[test]
fn a_cursor_past_the_newest_is_refused() {
    let backlog = filled(16, 6);
    assert_eq!(backlog.read(at(7), 100).expect_err("future").kind(), "daemon::invalid_frame");
    assert!(backlog.read(at(6), 100).expect("up to date").is_empty());
    assert_eq!(backlog.read(at(6_000), 100).expect_err("future").kind(), "daemon::invalid_frame");
}

#[test]
fn a_restart_reads_a_reset_then_the_new_generation_from_its_oldest() {
    let mut restarted = EventBacklog::new(RESTARTED, &LIMITS_DEFAULT);
    restarted.push(None, host(1), Vec::new()).expect("pushed");
    restarted.push(None, host(2), Vec::new()).expect("pushed");
    let frames = restarted.read(at(40), 100).expect("read");
    assert_eq!(frames.first(), Some(&EventFrame::GenerationReset { generation: RESTARTED, oldest: 1 }));
    assert_eq!(sequences(&frames), vec![1, 2]);
    let EventFrame::Record(record) = &frames[1] else { panic!("a record") };
    assert_eq!(record.generation, RESTARTED);
}

#[test]
fn an_ack_moves_receipt_alone() {
    let backlog = filled(16, 5);
    let ack = |sequence| EventAck { cursor: EventCursor { generation: GENERATION, sequence } };
    assert_eq!(backlog.ack(None, ack(3)).expect("acked"), EventCursor { generation: GENERATION, sequence: 3 });
    assert_eq!(backlog.ack(at(4), ack(2)).expect("acked").sequence, 4, "receipt never moves back");
    assert_eq!(backlog.ack(at(1), ack(6)).expect_err("future").kind(), "daemon::invalid_frame");
    let foreign = EventAck { cursor: EventCursor { generation: RESTARTED, sequence: 1 } };
    assert_eq!(backlog.ack(None, foreign).expect_err("foreign").kind(), "daemon::invalid_frame");
    // Receipt deletes nothing: an earlier cursor still reads every retained record.
    assert_eq!(sequences(&backlog.read(at(0), 100).expect("read")), vec![1, 2, 3, 4, 5]);
}

#[test]
fn the_backlog_holds_its_exact_record_and_byte_ceilings() {
    let mut records = backlog(4);
    for i in 1..=4 {
        records.push(None, host(i), Vec::new()).expect("pushed");
    }
    assert_eq!(records.oldest(), 1, "four held at a ceiling of four");
    records.push(None, host(5), Vec::new()).expect("pushed");
    assert_eq!(records.oldest(), 2, "the fifth drops the first");
    // Bytes: a ceiling of three records' bytes holds three, and a fourth drops one.
    let one = bitcode::encode(&EventsAnswer::Event(EventFrame::Record(Box::new(EventRecord {
        generation: GENERATION,
        sequence: 1,
        channel: None,
        payload: host(1),
        artifacts: Vec::new(),
    }))))
    .len() as u64;
    let mut bytes = EventBacklog::new(GENERATION, &Limits { event_bytes: 3 * one, ..LIMITS_DEFAULT });
    for i in 1..=3 {
        bytes.push(None, host(i), Vec::new()).expect("pushed");
    }
    assert_eq!(bytes.oldest(), 1, "three held within three records' bytes");
    bytes.push(None, host(4), Vec::new()).expect("pushed");
    assert_eq!(bytes.oldest(), 2);
    // A record past the event frame is refused, numbering nothing.
    let huge = EventPayload::Host(HostEvent {
        id: "huge".to_string(),
        from: "thread/fixture".to_string(),
        model: "fixture/thing/Changed".to_string(),
        event: NuVal::String("x".repeat(EVENT_FRAME_MAX as usize)),
        attached: None,
    });
    assert_eq!(bytes.push(None, huge, Vec::new()).expect_err("too large").kind(), "daemon::result_too_large");
    assert_eq!(bytes.newest(), Some(4));
}

/// A host event carrying a string of `len` bytes.
fn sized(len: usize) -> EventPayload {
    EventPayload::Host(HostEvent {
        id: "sized".to_string(),
        from: "thread/fixture".to_string(),
        model: "fixture/thing/Changed".to_string(),
        event: NuVal::String("x".repeat(len)),
        attached: None,
    })
}

/// The bytes the Events stream sends for a backlog's next record carrying `payload`.
fn envelope(payload: EventPayload) -> usize {
    bitcode::encode(&EventsAnswer::Event(EventFrame::Record(Box::new(EventRecord {
        generation: GENERATION,
        sequence: 1,
        channel: None,
        payload,
        artifacts: Vec::new(),
    }))))
    .len()
}

#[test]
fn the_ceiling_is_the_events_streams_own_envelope() {
    let max = EVENT_FRAME_MAX as usize;
    let mut len = max;
    while envelope(sized(len)) > max {
        len -= 1;
    }
    assert!(envelope(sized(len + 1)) > max, "the longest string that fits is {len} bytes");
    let mut backlog = backlog(4);
    backlog.push(None, sized(len), Vec::new()).expect("a record whose envelope fills the frame");
    let past = backlog.push(None, sized(len + 1), Vec::new()).expect_err("an envelope one past it");
    assert_eq!(past.kind(), "daemon::result_too_large");
    assert_eq!(backlog.newest(), Some(1), "the refused record took no sequence");
}

#[test]
fn a_record_its_subscriber_could_not_decode_is_refused() {
    let elements = usize::try_from(DECODE_BUDGET_DEFAULT.elements).expect("a usize");
    let wide = EventPayload::Host(HostEvent {
        id: "wide".to_string(),
        from: "thread/fixture".to_string(),
        model: "fixture/thing/Changed".to_string(),
        event: NuVal::List(vec![NuVal::Nothing; elements + 1]),
        attached: None,
    });
    assert!(envelope(wide.clone()) <= EVENT_FRAME_MAX as usize, "small on the wire");
    let mut backlog = backlog(4);
    let refused = backlog.push(None, wide, Vec::new()).expect_err("past the element budget");
    assert_ne!(refused.kind(), "daemon::result_too_large", "refused for its budget, not its size");
    assert_eq!(backlog.newest(), None, "the refused record took no sequence");
}

#[test]
fn an_epoch_change_with_events_pending_keeps_their_epoch_and_needs_a_fresh_ready() {
    let mut gate = ChannelGate::default();
    let epoch = |n| ChannelEpoch(n);
    gate.notice(&ChannelNotice::Opened { epoch: epoch(1), mode: ChannelMode::Async }).expect("opened");
    gate.notice(&ChannelNotice::Claimed { epoch: epoch(1) }).expect("claimed");
    gate.notice(&ChannelNotice::Ready { epoch: epoch(1) }).expect("ready");
    assert_eq!(gate.verified(), Some(epoch(1)));
    let mut backlog = backlog(16);
    backlog.push(gate.verified(), host(1), Vec::new()).expect("pushed");
    backlog.push(gate.verified(), host(2), Vec::new()).expect("pushed");
    // A new lease resets the Channel: nothing verified, the old epoch's ready refused.
    gate.reset();
    assert_eq!(gate.verified(), None);
    assert!(gate.notice(&ChannelNotice::Ready { epoch: epoch(1) }).is_err());
    gate.notice(&ChannelNotice::Opened { epoch: epoch(2), mode: ChannelMode::Sync }).expect("opened");
    assert!(gate.notice(&ChannelNotice::Ready { epoch: epoch(2) }).is_err(), "ready before a claim");
    gate.notice(&ChannelNotice::Claimed { epoch: epoch(2) }).expect("claimed");
    gate.notice(&ChannelNotice::Ready { epoch: epoch(2) }).expect("ready");
    assert_eq!(gate.verified(), Some(epoch(2)));
    // The pending events still say the epoch they were emitted under.
    for frame in backlog.read(None, 100).expect("read") {
        let EventFrame::Record(record) = frame else { panic!("a record") };
        assert_eq!(record.channel, Some(epoch(1)));
    }
}

#[test]
fn the_gate_refuses_a_notice_of_another_epoch_or_out_of_order() {
    let mut gate = ChannelGate::default();
    assert!(gate.notice(&ChannelNotice::Claimed { epoch: ChannelEpoch(1) }).is_err(), "a claim before an open");
    gate.notice(&ChannelNotice::Opened { epoch: ChannelEpoch(2), mode: ChannelMode::Async }).expect("opened");
    assert!(gate.notice(&ChannelNotice::Opened { epoch: ChannelEpoch(2), mode: ChannelMode::Async }).is_err(), "the same epoch again");
    assert!(gate.notice(&ChannelNotice::Opened { epoch: ChannelEpoch(1), mode: ChannelMode::Async }).is_err(), "an older epoch");
    assert!(gate.notice(&ChannelNotice::Claimed { epoch: ChannelEpoch(1) }).is_err(), "another epoch's claim");
    gate.notice(&ChannelNotice::Closed { epoch: ChannelEpoch(2), code: 1000, reason: "done".to_string() }).expect("closed");
    assert_eq!(gate.state(), (ChannelEpoch(2), ChannelPhase::Closed, false));
}

#[test]
fn a_slow_subscriber_reads_a_gap_then_is_closed_past_its_window() {
    let backlog = filled(4, 6);
    match backlog.deliver(at(2), 8, 100).expect("delivered") {
        Delivery::Frames(frames) => assert_eq!(sequences(&frames), vec![3, 4, 5, 6]),
        closed => panic!("closed within its window: {closed:?}"),
    }
    match backlog.deliver(at(1), 8, 100).expect("delivered") {
        Delivery::Frames(frames) => assert_eq!(frames.first(), Some(&EventFrame::Gap { from: 2, to: 2 })),
        closed => panic!("closed within its window: {closed:?}"),
    }
    let far = filled(4, 20);
    assert_eq!(far.deliver(at(2), 8, 100).expect("delivered"), Delivery::Close { behind: 18 });
    // A fresh subscription, or another generation's, isn't held to the window.
    assert!(matches!(far.deliver(None, 8, 100).expect("delivered"), Delivery::Frames(_)));
}

/// Every event message variant's bitcode.
fn vectors() -> BTreeMap<String, golden::Vector> {
    let artifact = EventArtifact {
        artifact: ArtifactRef { id: ArtifactId([0x22; 16]), size: 4096, sha256: [0x33; 32] },
        dest: "out/report.nuon".to_string(),
    };
    let record = |payload, artifacts| EventRecord { generation: GENERATION, sequence: 7, channel: Some(ChannelEpoch(2)), payload, artifacts };
    let emergency = EventPayload::Emergency(Emergency::CpuWarning(CpuWarningEmergency { cpu_pct: 97.5, sample_ms: 1000 }));
    let lease = EventPayload::Lease(LeaseNotice::Lost { lease: LeaseId([1; 16]), epoch: LeaseEpoch(3), reason: LossReason::Expired });
    let mut out = BTreeMap::new();
    let mut put = |name: &str, vector: golden::Vector| {
        out.insert(name.to_string(), vector);
    };
    put("cursor", golden::vector(&EventCursor { generation: GENERATION, sequence: 7 }));
    put("frame/record/host", golden::vector(&EventFrame::Record(Box::new(record(host(1), vec![artifact.clone()])))));
    put("frame/record/emergency", golden::vector(&EventFrame::Record(Box::new(record(emergency, Vec::new())))));
    put("frame/record/lease", golden::vector(&EventFrame::Record(Box::new(record(lease, Vec::new())))));
    put("frame/gap", golden::vector(&EventFrame::Gap { from: 2, to: 5 }));
    put("frame/generation_reset", golden::vector(&EventFrame::GenerationReset { generation: RESTARTED, oldest: 1 }));
    put("resume/cursor", golden::vector(&EventResume { cursor: at(7) }));
    put("resume/oldest", golden::vector(&EventResume { cursor: None }));
    put("ack", golden::vector(&EventAck { cursor: EventCursor { generation: GENERATION, sequence: 7 } }));
    out
}

#[test]
fn each_message_has_its_golden_vector_and_round_trips() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an event message's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let vectors = vectors();
    let frame: EventFrame = decode_payload(&vectors["frame/record/host"].bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    let EventFrame::Record(record) = frame else { panic!("a record") };
    assert_eq!(record.artifacts.len(), 1);
    assert_eq!(record.artifacts[0].dest, "out/report.nuon");
    for (name, vector) in &vectors {
        assert!(vector.bytes.len() <= EVENT_FRAME_MAX as usize, "{name} fits an event frame");
    }
}
