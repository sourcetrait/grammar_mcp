//! The remote Channel across the engine boundary: the seven commands' typed
//! results held to the catalog, the lifecycle and delivery events in their
//! 0.0.0-9 shapes, a relayed packet kept as data with its files' association,
//! no peer field reaching an operation, a worker, an alias, or a root, and
//! golden vectors.

use std::collections::{BTreeMap, BTreeSet};

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/peer.nuon";

const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const NOM: &str = "peernom0001";

fn text(s: &str) -> NuVal {
    NuVal::String(s.to_string())
}

fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(key, value)| (key.to_string(), value.clone())).collect())
}

fn row() -> RemoteChannelRow {
    RemoteChannelRow {
        alias: "lab/peer".to_string(),
        address: Some("127.0.0.1:7000".to_string()),
        listen: None,
        cert: Some("peer.pem".to_string()),
        state: RemoteLinkState::Open,
        endpoint: Some("127.0.0.1:7000".to_string()),
        remote_mcp_nom: Some(NOM.to_string()),
    }
}

fn failure() -> RemoteFailure {
    RemoteFailure { kind: "listen".to_string(), message: "address in use".to_string() }
}

fn packet(model: &str, event: NuVal) -> RemotePacket {
    RemotePacket { delivery: "eventid0001".to_string(), origin: NOM.to_string(), model: model.to_string(), event }
}

/// Every result shape a remote command answers.
fn results() -> Vec<(&'static str, RemoteResult)> {
    vec![
        ("rows", RemoteResult::Rows(vec![row()])),
        ("row", RemoteResult::Row(row())),
        ("listening", RemoteResult::Listening(RemoteListenResult { endpoint: "127.0.0.1:7000".to_string() })),
        ("done", RemoteResult::Done),
    ]
}

/// Every lifecycle and delivery event, each shape of `Disconnected` among them.
fn events() -> Vec<(&'static str, RemoteChannelEvent)> {
    use RemoteChannelEvent as E;
    vec![
        ("connected", E::Connected { remote: "lab/peer".to_string(), mcp_nom: NOM.to_string() }),
        ("disconnected/closed", E::Disconnected { remote: "lab/peer".to_string(), mcp_nom: None, error: None }),
        ("disconnected/ended", E::Disconnected { remote: "lab/peer".to_string(), mcp_nom: Some(NOM.to_string()), error: None }),
        ("disconnected/failed", E::Disconnected { remote: "lab/peer".to_string(), mcp_nom: None, error: Some(failure()) }),
        ("sent", E::Sent { id: "eventid0001".to_string(), mcp_nom: NOM.to_string() }),
        ("unsent", E::Unsent { id: "eventid0001".to_string(), mcp_nom: NOM.to_string(), error: failure() }),
    ]
}

#[test]
fn each_remote_command_has_one_result_shape_its_catalog_entry_declares() {
    let catalog: Vec<_> = COMMANDS.iter().filter(|spec| spec.words().starts_with("remote channel")).collect();
    assert_eq!(catalog.iter().map(|spec| spec.kind).collect::<Vec<_>>(), REMOTE_COMMANDS);
    for spec in &catalog {
        let fits: Vec<_> = results().into_iter().filter(|(_, result)| result.answers(spec.kind)).collect();
        assert_eq!(fits.len(), 1, "{}", spec.words());
        let declared = match fits[0].1 {
            RemoteResult::Rows(_) => "table",
            RemoteResult::Row(_) | RemoteResult::Listening(_) => "record",
            RemoteResult::Done => "nothing",
        };
        assert_eq!(spec.result, declared, "{}", spec.words());
    }
    for spec in COMMANDS.iter().filter(|spec| !REMOTE_COMMANDS.contains(&spec.kind)) {
        assert!(results().iter().all(|(_, result)| !result.answers(spec.kind)), "{}", spec.words());
    }
    // A list or a get reads; the other five change the worker's links or its file.
    for spec in &catalog {
        let class = request_class(&RuntimeRequest::Interface { command: spec.name.to_string() });
        let reads = matches!(spec.kind, CommandKind::RemoteList | CommandKind::RemoteGet);
        assert_eq!(class, if reads { RequestClass::Query } else { RequestClass::Mutating }, "{}", spec.words());
    }
}

#[test]
fn a_row_and_a_listen_render_as_the_tools_always_have() {
    let none = NuVal::Nothing;
    assert_eq!(
        nuval::to_nuval(&row()).expect("a value"),
        record(&[
            ("alias", text("lab/peer")),
            ("address", text("127.0.0.1:7000")),
            ("listen", none),
            ("cert", text("peer.pem")),
            ("state", text("open")),
            ("endpoint", text("127.0.0.1:7000")),
            ("remote_mcp_nom", text(NOM)),
        ])
    );
    assert_eq!(
        nuval::to_nuval(&RemoteListenResult { endpoint: "127.0.0.1:7000".to_string() }).expect("a value"),
        record(&[("endpoint", text("127.0.0.1:7000"))])
    );
}

#[test]
fn each_event_carries_its_packets_0_0_0_9_record_and_model() {
    let error = record(&[("kind", text("listen")), ("message", text("address in use"))]);
    let shapes = [
        (MODEL_CONNECTED, record(&[("remote", text("lab/peer")), ("mcp_nom", text(NOM))])),
        (MODEL_DISCONNECTED, record(&[("remote", text("lab/peer"))])),
        (MODEL_DISCONNECTED, record(&[("remote", text("lab/peer")), ("mcp_nom", text(NOM))])),
        (MODEL_DISCONNECTED, record(&[("remote", text("lab/peer")), ("error", error.clone())])),
        (MODEL_SENT, record(&[("id", text("eventid0001")), ("mcp_nom", text(NOM))])),
        (MODEL_UNSENT, record(&[("id", text("eventid0001")), ("mcp_nom", text(NOM)), ("error", error)])),
    ];
    for ((name, event), (model, shape)) in events().into_iter().zip(shapes) {
        assert_eq!(event.model(), model, "{name}");
        assert_eq!(event.event(), shape, "{name}: an absent key left out, never null");
    }
}

#[test]
fn a_peer_id_is_base62_alone() {
    let max = "a".repeat(PEER_ID_MAX);
    for good in [NOM, "eventid0001", "7", max.as_str()] {
        assert!(valid_peer_id(good), "{good}");
    }
    let long = "a".repeat(PEER_ID_MAX + 1);
    for bad in ["", "..", "../x", "/srv/other", "a/b", "a.b", "a b", "a\\b", "nöm", "id\0", long.as_str()] {
        assert!(!valid_peer_id(bad), "{bad:?}");
    }
}

#[test]
fn a_relayed_packet_is_held_to_a_relays_rules() {
    let ok = packet("peer/Checkpoint", record(&[("n", NuVal::Int(1))]));
    ok.check().expect("an ordinary relay");
    assert_eq!(ok.packet_from(), format!("remote/channel/{NOM}"));
    let refusals = [
        ("a host model", packet("mcp/channel/Open", NuVal::Nothing)),
        ("a host's remote report", packet(MODEL_SENT, NuVal::Nothing)),
        ("no model", packet("", NuVal::Nothing)),
        ("an origin that escapes", RemotePacket { origin: "../..".to_string(), ..packet("peer/Note", NuVal::Nothing) }),
        ("a delivery that escapes", RemotePacket { delivery: "/srv/other".to_string(), ..packet("peer/Note", NuVal::Nothing) }),
    ];
    for (name, packet) in refusals {
        assert_eq!(packet.check().expect_err(name).kind(), "daemon::invalid_frame", "{name}");
    }
}

#[test]
fn a_relay_keeps_its_files_association_and_its_scope() {
    let mut backlog = EventBacklog::new(GENERATION, &LIMITS_DEFAULT);
    let report = EventArtifact { artifact: ArtifactRef { id: ArtifactId([0x22; 16]), size: 4096, sha256: [0x33; 32] }, dest: "out/report.nuon".to_string() };
    let spill = EventArtifact { artifact: ArtifactRef { id: ArtifactId([0x23; 16]), size: 2048, sha256: [0x34; 32] }, dest: EVENT_SPILL_DEST.to_string() };
    let event = record(&[(EVENT_SPILL_KEY, text(EVENT_SPILL_DEST)), (EVENT_BYTES_KEY, NuVal::Int(2048))]);
    let relayed = EventPayload::Relayed(packet("peer/Report", event.clone()));
    let sequence = backlog.push(Some(ChannelEpoch(3)), relayed, vec![report.clone(), spill.clone()]).expect("numbered");
    let frames = backlog.read(None, 10).expect("read");
    let [EventFrame::Record(record)] = frames.as_slice() else { panic!("one record") };
    assert_eq!((record.sequence, record.generation, record.channel), (sequence, GENERATION, Some(ChannelEpoch(3))));
    let EventPayload::Relayed(back) = &record.payload else { panic!("a relay") };
    assert_eq!((back.origin.as_str(), back.delivery.as_str(), back.model.as_str()), (NOM, "eventid0001", "peer/Report"));
    assert_eq!(back.event, event, "the spill pointer names a dest among the record's own files");
    assert_eq!(record.artifacts, vec![report, spill]);
}

#[test]
fn no_peer_event_poses_as_the_hosts_or_reaches_an_operation() {
    let mut backlog = EventBacklog::new(GENERATION, &LIMITS_DEFAULT);
    let host = |from: &str, model: &str| {
        EventPayload::Host(HostEvent { id: "eventid0002".to_string(), from: from.to_string(), model: model.to_string(), event: NuVal::Nothing, attached: None })
    };
    let posing = [
        ("a relay as a host event", host(&format!("remote/channel/{NOM}"), "peer/Note")),
        ("a host report as a host event", host("mcp", MODEL_SENT)),
        ("a relay of a host model", EventPayload::Relayed(packet("mcp/channel/Open", NuVal::Nothing))),
    ];
    for (name, payload) in posing {
        assert_eq!(backlog.push(None, payload, Vec::new()).expect_err(name).kind(), "daemon::invalid_frame", "{name}");
    }
    assert_eq!(backlog.newest(), None, "nothing numbered");
    // A payload shaped like an engine call is data: held and read back as sent.
    let shaped = record(&[
        ("nonce", text("aaaaaaaaaaa")),
        ("definition", text("def execute [data: nothing]: nothing -> nothing { }")),
        ("host", text("lab")),
        ("lease", text("0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a")),
    ]);
    backlog.push(None, EventPayload::Relayed(packet("peer/Execute", shaped.clone())), Vec::new()).expect("numbered as data");
    let frames = backlog.read(None, 10).expect("read");
    let [EventFrame::Record(record)] = frames.as_slice() else { panic!("one record") };
    let EventPayload::Relayed(back) = &record.payload else { panic!("a relay") };
    assert_eq!(back.event, shaped);
    // What runs is a submission under a ticket the lease's own stream was issued:
    // the call the event describes, submitted without one, admits nothing.
    let nonce = sourcetrait_common::datum::Nonce::new_unchecked(7);
    let request = RuntimeRequest::Engine(ToEngineSys::Call(CallRequest {
        host: Host::Local,
        nonce,
        namepath: "gram:get:log".to_string(),
        data: "{refs: []}".to_string(),
        timeout: None,
    }));
    let forged = OperationTicket { operation: OperationId([0x11; 16]), nonce, generation: GENERATION, epoch: LeaseEpoch(1), expires_ms: 60_000 };
    let submission = Submission { correlation: 1, ticket: forged, hash: payload_hash(&request), request };
    let (admission, execute) = OperationTable::new(GENERATION).submit(0, &submission, LeaseEpoch(1));
    assert!(matches!(admission.outcome, AdmissionOutcome::Refused(TransportError::TicketInvalid { .. })));
    assert_eq!(execute, None);
}

/// Every peer message's bitcode.
fn vectors() -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for (name, result) in results() {
        out.insert(format!("result/{name}"), bitcode::encode(&RuntimeReply::Remote(result)));
    }
    for (name, event) in events() {
        out.insert(format!("event/{name}"), bitcode::encode(&event));
    }
    let relayed = packet("peer/Checkpoint", record(&[("event_bytes", NuVal::Int(641))]));
    out.insert("packet".to_string(), bitcode::encode(&relayed));
    let frame = |payload| {
        EventFrame::Record(Box::new(EventRecord { generation: GENERATION, sequence: 9, channel: Some(ChannelEpoch(2)), payload, artifacts: Vec::new() }))
    };
    out.insert("frame/remote".to_string(), bitcode::encode(&frame(EventPayload::Remote(events()[0].1.clone()))));
    out.insert("frame/relayed".to_string(), bitcode::encode(&frame(EventPayload::Relayed(relayed))));
    out
}

#[test]
fn each_message_has_its_golden_vector_and_round_trips() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "a peer message's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let covered: BTreeSet<_> = results()
        .iter()
        .map(|(_, result)| match result {
            RemoteResult::Rows(_) => "rows",
            RemoteResult::Row(_) => "row",
            RemoteResult::Listening(_) => "listening",
            RemoteResult::Done => "done",
        })
        .collect();
    assert_eq!(covered.len(), 4);
    for (name, result) in results() {
        let reply = RuntimeReply::Remote(result);
        let back: RuntimeReply = decode_payload(&bitcode::encode(&reply), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, reply, "{name}");
    }
    for (name, event) in events() {
        let back: RemoteChannelEvent = decode_payload(&bitcode::encode(&event), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, event, "{name}");
    }
    for (name, bytes) in vectors() {
        assert!(bytes.len() <= EVENT_FRAME_MAX as usize, "{name} fits an event frame");
    }
}
