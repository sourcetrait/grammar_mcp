//! The runtime's data on the wire: the interface catalog held to its baseline, the
//! ownership table held to each dispatch, a golden bitcode vector for every
//! operator, Channel, peer, runtime, transport, and artifact variant, their round
//! trips, and the errors' kinds and rows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use sourcetrait_common::datum::{Nom, Nonce};
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// A payload as `T` within the default decode budget, the path every frame takes.
fn decode<T: bitcode::DecodeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    decode_payload(payload, DECODE_BUDGET_DEFAULT)
}

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/runtime.nuon";

const OPERATION: OperationId = OperationId([0x11; 16]);
const ARTIFACT: ArtifactId = ArtifactId([0x22; 16]);
const HASH: [u8; 32] = [0x33; 32];

fn warning() -> Diagnostic {
    Diagnostic::warning("lint::summary_length", None, "a summary past its length")
}

fn refusal() -> Diagnostic {
    let source = Source { path: None, position: [1, 10] };
    Diagnostic::error("interface::invalid", Some(source), "one `interface` command, alone")
}

/// A command kind, matched exhaustively so a new one fails the build here.
fn kind_name(kind: CommandKind) -> &'static str {
    use CommandKind as K;
    match kind {
        K::Info => "info",
        K::Help => "help",
        K::ProcessList => "process list",
        K::ProcessKill => "process kill",
        K::PurviewList => "purview list",
        K::PurviewSet => "purview set",
        K::PurviewExtend => "purview extend",
        K::PurviewConfigure => "purview configure",
        K::ChannelOpen => "channel open",
        K::ChannelSync => "channel sync",
        K::ChannelReady => "channel ready",
        K::ChannelClose => "channel close",
        K::ChannelConfig => "channel config",
        K::RemoteList => "remote channel list",
        K::RemoteGet => "remote channel get",
        K::RemoteSet => "remote channel set",
        K::RemoteUnset => "remote channel unset",
        K::RemoteListen => "remote channel listen",
        K::RemoteConnect => "remote channel connect",
        K::RemoteClose => "remote channel close",
        K::EquipNew => "equip new",
        K::EquipInstall => "equip install",
        K::EquipCheck => "equip check",
        K::EquipUninstall => "equip uninstall",
        K::EquipScaffold => "equip scaffold",
        K::EquipCommit => "equip commit",
    }
}

#[test]
fn the_catalog_holds_the_baseline_commands() {
    assert_eq!(COMMANDS.len(), 26);
    let words: BTreeSet<&str> = COMMANDS.iter().map(|spec| spec.words()).collect();
    assert_eq!(words.len(), 26, "a command's words repeat");
    for spec in COMMANDS {
        assert_eq!(spec.words(), kind_name(spec.kind), "{}", spec.name);
        assert_eq!(spec.name, format!("{INTERFACE} {}", spec.words()));
        assert!(spec.summary.len() <= 80, "{}: a summary past 80", spec.name);
        assert!(["record", "table", "nothing"].contains(&spec.result), "{}", spec.name);
        assert!(command_spec(spec.name).is_some_and(|found| found.kind == spec.kind));
        assert!(command_by_words(spec.words()).is_some_and(|found| found.kind == spec.kind));
    }
    let remote: Vec<&str> =
        COMMANDS.iter().filter(|spec| spec.topic == "Remote").map(|spec| spec.words()).collect();
    assert_eq!(remote.len(), 7, "the seven remote channel commands");
    let topics: Vec<&str> = COMMANDS.iter().map(|spec| spec.topic).fold(Vec::new(), |mut seen, topic| {
        if seen.last() != Some(&topic) {
            seen.push(topic);
        }
        seen
    });
    assert_eq!(topics, ["General", "Process", "Purview", "Channel", "Remote", "Equip"]);
    assert_eq!(HELP_WORDS, ["interface", "grimm"]);
}

/// The ownership table's rows by surface, each of six cells.
fn ownership() -> BTreeMap<String, Vec<String>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../doc/ai/daemon/runtime_ownership.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    text.lines()
        .skip(1)
        .map(|line| {
            let cells: Vec<String> = line.split('\t').map(String::from).collect();
            assert_eq!(cells.len(), 6, "{line}");
            (cells[0].clone(), cells)
        })
        .collect()
}

#[test]
fn every_command_and_tool_has_an_owner() {
    let rows = ownership();
    for spec in COMMANDS {
        let row = rows.get(spec.name).unwrap_or_else(|| panic!("{} has no owner", spec.name));
        assert!(["engine", "runtime", "frontend"].contains(&row[3].as_str()), "{}: {}", spec.name, row[3]);
    }
    for tool in ["execute", "rerun", "call", "interface"] {
        assert!(rows.contains_key(tool), "the {tool} tool has no owner");
    }
    for callback in ["channel_send", "remote_send", "config_record", "pin_config"] {
        assert!(rows.contains_key(callback), "the {callback} callback has no owner");
    }
}

#[test]
fn the_ownership_table_keeps_each_dispatch() {
    let rows = ownership();
    let help = &rows["interface help"];
    assert_eq!((help[3].as_str(), help[4].as_str()), ("runtime", "RuntimeRequest::Interface"));
    for word in HELP_WORDS {
        let branch = rows
            .get(&format!("interface help {word}"))
            .unwrap_or_else(|| panic!("help's reserved word {word} has no branch"));
        assert!(branch[5].starts_with("InspectDoc::Signatures"), "{word}: {}", branch[5]);
    }
    assert_eq!(rows["interface help <namepath>"][4], "RuntimeRequest::Engine(Control(EngineCommand::Help))");
    assert!(
        rows["interface help <pattern>"][4].starts_with("RuntimeRequest::Engine(Control(EngineCommand::Signatures))"),
        "{}",
        rows["interface help <pattern>"][4]
    );
    for callback in ["channel_send", "remote_send"] {
        assert!(rows[callback][5].contains("id, or the refusal's text"), "{callback}: {}", rows[callback][5]);
    }
    assert_eq!(rows["config_record"][4], "none: local to the worker");
}

#[test]
fn a_denied_command_leaves_both_blocks() {
    let all_list = render_command_list(|_| false);
    let all_signatures = render_signatures(|_| false);
    for spec in COMMANDS {
        assert!(all_list.contains(spec.name), "{}", spec.name);
        assert!(all_signatures.contains(&spec.signature_text()), "{}", spec.name);
        let list = render_command_list(|words| words == spec.words());
        let signatures = render_signatures(|words| words == spec.words());
        assert!(!list.contains(&format!("- {}\n", spec.name)), "{}", spec.name);
        assert!(!signatures.contains(&format!("`{}`", spec.signature_text())), "{}", spec.name);
    }
}

#[test]
fn a_signature_line_names_each_parameter_and_the_result() {
    let set = command_by_words("remote channel set").expect("set");
    assert_eq!(
        set.signature_text(),
        "remote channel set [alias: string, --address: string, --listen: string, --cert: string]: nothing -> record",
    );
    let help = command_by_words("help").expect("help");
    assert_eq!(help.signature_text(), "help [namepath: string]: nothing -> record");
}

/// Every operator vector.
fn operator() -> Vec<(String, Emergency)> {
    let all = vec![
        Emergency::HungEngineThread(HungEngineThreadEmergency {
            nonce: "aOiwlfCpNKg".into(),
            lane: "stateless".into(),
            tool: "execute".into(),
            started_at: 1,
            cancelled_at: 2,
            hung_ms: u64::MAX,
            pool_held: u64::MAX,
            pool_cap: 16,
        }),
        Emergency::CpuWarning(CpuWarningEmergency { cpu_pct: 812.5, sample_ms: 5000 }),
        Emergency::RamWarning(RamWarningEmergency { rss_kb: u64::MAX }),
        Emergency::VramWarning(VramWarningEmergency { used_mib: 24_000, total_mib: 24_576 }),
        Emergency::BackgroundJobsWarning(BackgroundJobsWarningEmergency { job_count: u64::MAX }),
        Emergency::DiskWarning(DiskWarningEmergency { mount: "/".into(), used_pct: 95, threshold_pct: 90 }),
        Emergency::ChannelSpamWarning(ChannelSpamWarningEmergency {
            origin: "thread/aOiwlfCpNKg".into(),
            hits: 21,
            window_secs: 1,
            rate: 20,
        }),
        Emergency::ChannelSpamError(ChannelSpamErrorEmergency {
            origin: "thread/aOiwlfCpNKg".into(),
            hits: 101,
            window_secs: 10,
            rate: 100,
            action: "stopped the eval".into(),
        }),
        Emergency::Critical(CriticalEmergency { reason: "every permit held".into(), hung: 16, cap: 16 }),
    ];
    all.into_iter().map(|e| (format!("emergency/{}", e.kind().name()), e)).collect()
}

/// Every runtime error, by its variant.
fn runtime_errors() -> Vec<(&'static str, RuntimeError)> {
    use RuntimeError as R;
    vec![
        ("interface_violations", R::InterfaceViolations { diagnostics: vec![refusal(), warning()] }),
        ("host_unknown", R::HostUnknown { host: "example".into() }),
        ("schema_invalid", R::SchemaInvalid { reason: "not a type".into() }),
        ("purview_invalid_id", R::PurviewInvalidId { id: "Bad".into(), reason: "uppercase".into() }),
        ("channel_start", R::ChannelStart { reason: "no port".into() }),
        ("channel_not_open", R::ChannelNotOpen),
        ("channel_not_claimed", R::ChannelNotClaimed { mode: ChannelMode::Sync }),
        ("channel_wrong_mode", R::ChannelWrongMode { open: ChannelMode::Async, asked: ChannelMode::Sync }),
        ("channel_peer_gone", R::ChannelPeerGone),
        ("remote_invalid_params", R::RemoteInvalidParams { reason: "no cert".into() }),
        ("remote_already_open", R::RemoteAlreadyOpen { alias: "lab/peer".into() }),
        ("remote_not_open", R::RemoteNotOpen { alias: "lab/peer".into() }),
        ("remote_open_failed", R::RemoteOpenFailed { alias: "lab/peer".into(), reason: "refused".into() }),
        ("remote_channel_unverified", R::RemoteChannelUnverified),
        ("remotes_file", R::RemotesFile { reason: "line 3".into() }),
        ("internal", R::Internal { phase: "io".into(), reason: "denied".into() }),
        ("artifact", R::Artifact(ArtifactError::Unknown { id: ARTIFACT })),
    ]
}

/// Every transport error, by its variant.
fn transport_errors() -> Vec<(&'static str, TransportError)> {
    use TransportError as T;
    let mismatch = || ("1.0".to_string(), "2.0".to_string());
    vec![
        ("version_mismatch", T::VersionMismatch { expected: mismatch().0, found: mismatch().1 }),
        ("schema_mismatch", T::SchemaMismatch { expected: "a".into(), found: "b".into() }),
        ("codec_mismatch", T::CodecMismatch { expected: "bitcode-0.6.9".into(), found: "bitcode-0.7.0".into() }),
        ("wrong_worker", T::WrongWorker { expected: "fixture/default".into(), found: "fixture/test".into() }),
        ("unsupported_feature", T::UnsupportedFeature { feature: "artifacts".into() }),
        ("operation_conflict", T::OperationConflict { operation: OPERATION }),
        ("ticket_invalid", T::TicketInvalid { reason: "expired".into() }),
        ("unauthorized", T::Unauthorized { reason: "peer uid 1001".into() }),
        ("authentication_failed", T::AuthenticationFailed { reason: "publickey".into() }),
        ("lease_invalid", T::LeaseInvalid { reason: "another epoch".into() }),
        ("lease_expired", T::LeaseExpired),
        ("busy", T::Busy),
        ("invalid_frame", T::InvalidFrame { reason: "a trailing byte".into() }),
        ("result_too_large", T::ResultTooLarge { limit: 8 << 20 }),
        ("unknown_outcome", T::UnknownOutcome { operation: OPERATION, reason: "the stream closed".into() }),
        ("unavailable", T::Unavailable { reason: "no route".into() }),
    ]
}

fn epoch() -> ChannelEpoch {
    ChannelEpoch(u64::MAX)
}

fn description() -> ArtifactDescription {
    ArtifactDescription {
        artifact: ArtifactRef { id: ARTIFACT, size: ARTIFACT_MAX_BYTES, sha256: HASH },
        purpose: ArtifactPurpose::OperationResult,
        display_name: "result ✓.nuon".into(),
        sftp_path: RemotePath::new("/srv/runs/objects/22222222222222222222222222222222").expect("a remote path"),
    }
}

fn requests() -> Vec<(&'static str, RuntimeRequest)> {
    let call = CallRequest {
        host: Host::Local,
        nonce: Nonce::new_unchecked(7),
        namepath: "gram:get:log".into(),
        data: "{refs: []}".into(),
        timeout: None,
    };
    let reserve = ReserveUpload {
        purpose: ArtifactPurpose::EquipSource,
        display_name: "geo.tar".into(),
        size: 1,
        sha256: HASH,
    };
    vec![
        ("engine", RuntimeRequest::Engine(call.into())),
        ("interface", RuntimeRequest::Interface { command: "interface purview set ['core']".into() }),
        ("info", RuntimeRequest::Info),
        ("operation_lookup", RuntimeRequest::OperationLookup { operation: OPERATION }),
        (
            "policy",
            RuntimeRequest::Policy(ControllerPolicy {
                deny_tools: vec!["rerun".into()],
                deny_commands: vec!["equip uninstall".into()],
            }),
        ),
        ("channel", RuntimeRequest::Channel(ChannelNotice::Opened { epoch: epoch(), mode: ChannelMode::Async })),
        ("artifact", RuntimeRequest::Artifact(ArtifactRequest::Reserve(reserve))),
        ("cancel", RuntimeRequest::Cancel { target: OPERATION }),
    ]
}

fn replies() -> Vec<(&'static str, RuntimeReply)> {
    let info = InfoReply {
        name: "grammar".into(),
        version: "0.0.0-9".into(),
        nu_version: "0.116.1".into(),
        id: "fixture".into(),
        namespace: "default".into(),
        nom: "localnom001".into(),
        work_dir: "/srv/work".into(),
        tmp: "/srv/tmp/grammard.x".into(),
        transient: "/run/user/2001/grammard.y".into(),
        plugins: vec![PluginInfo("nu_plugin_eye".into(), Some("0.1.0".into())), PluginInfo("bad".into(), None)],
        signatures: "gram\n get # read\n".into(),
        purview: vec![PurviewView("core".into(), vec!["gram:".into()])],
    };
    let answer = ValueAnswer { nonce: Nonce::new_unchecked(7), result: Ok(NuVal::Nothing) };
    vec![
        ("engine", RuntimeReply::Engine(FromEngineSys::Value(answer))),
        ("interface", RuntimeReply::Interface { result: NuVal::Record(vec![("ok".into(), NuVal::Bool(true))]) }),
        ("info", RuntimeReply::Info(info)),
        ("operation", RuntimeReply::Operation(OperationStatus::Known { operation: OPERATION, state: OperationState::Running })),
        ("policy", RuntimeReply::Policy(ControllerPolicy { deny_tools: Vec::new(), deny_commands: Vec::new() })),
        (
            "channel",
            RuntimeReply::Channel(ChannelState {
                phase: ChannelPhase::Verified,
                mode: Some(ChannelMode::Sync),
                epoch: epoch(),
                claimed: true,
                sync_dropped: 3,
            }),
        ),
        ("artifact", RuntimeReply::Artifact(ArtifactReply::Described(description()))),
        ("refused", RuntimeReply::Refused(RuntimeError::ChannelNotOpen)),
    ]
}

fn channel() -> Vec<(&'static str, ChannelNotice)> {
    vec![
        ("opened", ChannelNotice::Opened { epoch: epoch(), mode: ChannelMode::Sync }),
        ("claimed", ChannelNotice::Claimed { epoch: epoch() }),
        ("ready", ChannelNotice::Ready { epoch: epoch() }),
        ("closed", ChannelNotice::Closed { epoch: ChannelEpoch(0), code: 1008, reason: "emit before verification".into() }),
    ]
}

fn peer() -> Vec<(&'static str, RemoteChannelEvent)> {
    let failure = || RemoteFailure { kind: "listen".into(), message: "tls".into() };
    vec![
        ("connected", RemoteChannelEvent::Connected { remote: "lab/peer".into(), mcp_nom: "peernom0001".into() }),
        ("disconnected_closed", RemoteChannelEvent::Disconnected { remote: "lab/peer".into(), mcp_nom: None, error: None }),
        ("disconnected_failed", RemoteChannelEvent::Disconnected { remote: "lab/peer".into(), mcp_nom: None, error: Some(failure()) }),
        ("disconnected_ended", RemoteChannelEvent::Disconnected { remote: "lab/peer".into(), mcp_nom: Some("peernom0001".into()), error: None }),
        ("sent", RemoteChannelEvent::Sent { id: "eventid0001".into(), mcp_nom: "peernom0001".into() }),
        ("unsent", RemoteChannelEvent::Unsent { id: "eventid0001".into(), mcp_nom: "peernom0001".into(), error: failure() }),
    ]
}

fn artifacts() -> Vec<(&'static str, ArtifactRequest, ArtifactReply)> {
    let reserve = ReserveUpload { purpose: ArtifactPurpose::ChannelAttachment, display_name: String::new(), size: 0, sha256: HASH };
    let log = ReadLog { operation: OPERATION, stream: LogStream::Stderr, offset: u64::MAX, max_bytes: LOG_READ_MAX };
    let chunk = LogChunk { generation: WorkerGeneration([0x44; 16]), offset: 10, bytes: vec![0, 0xff], eof: true, done: false };
    let staging = RemotePath::new("/srv/state/staging/22222222222222222222222222222222").expect("a remote path");
    let reservation = UploadReservation { id: ARTIFACT, staging_path: staging, expires_ms: u32::MAX };
    let publishing = ArtifactStatus::Known { id: ARTIFACT, state: ArtifactState::Publishing };
    vec![
        ("reserve", ArtifactRequest::Reserve(reserve), ArtifactReply::Reserved(reservation)),
        ("publish", ArtifactRequest::Publish(PublishUpload { id: ARTIFACT }), ArtifactReply::Published(description())),
        ("inspect", ArtifactRequest::Inspect(InspectArtifact { id: ARTIFACT }), ArtifactReply::Inspected(publishing)),
        ("abort", ArtifactRequest::Abort(AbortUpload { id: ARTIFACT }), ArtifactReply::Aborted { id: ARTIFACT }),
        ("describe", ArtifactRequest::Describe(DescribeArtifact { id: ARTIFACT }), ArtifactReply::Described(description())),
        ("read_log", ArtifactRequest::ReadLog(log), ArtifactReply::Log(chunk)),
    ]
}

fn misc() -> Vec<(&'static str, Vec<u8>)> {
    let observation = Observation {
        engine: Some(EngineHealth {
            hung: vec![HungEval { nonce: "n".into(), tool: "call".into(), lane: Lane::Stateful, started_at: 1, cancelled_at: 2 }],
            permits_held: 1,
            permits_cap: 16,
            jobs: 0,
        }),
        listening: true,
    };
    let event = HostEvent {
        id: "eventid0002".into(),
        from: "remote/channel/peernom0001".into(),
        model: "peer/Checkpoint".into(),
        event: NuVal::Record(vec![("event_bytes".into(), NuVal::Int(641))]),
        attached: Some("peernom0001/attach00001".into()),
    };
    let send = ChannelSendData { from: "thread/x".into(), model: "example/Note".into(), event: NuVal::Nothing, attached: None };
    let remote = RemoteSendData { mcp_nom: "peernom0001".into(), model: "example/Receipt".into(), event: NuVal::Nothing, attached: Some(NuVal::List(Vec::new())) };
    let row = RemoteChannelRow {
        alias: "lab/peer".into(),
        address: None,
        listen: Some("127.0.0.1:7000".into()),
        cert: Some("peer.pem".into()),
        state: RemoteLinkState::Open,
        endpoint: Some("127.0.0.1:7000".into()),
        remote_mcp_nom: Some("peernom0001".into()),
    };
    let thresholds = SpamThresholdsData { warn_window_secs: 1, warn_rate: 20, error_window_secs: 10, error_rate: 100 };
    let reference = ArtifactRef { id: ARTIFACT, size: 1, sha256: HASH };
    let _ = Nom::new_unchecked(0);
    vec![
        ("observation", bitcode::encode(&observation)),
        ("host_event", bitcode::encode(&event)),
        ("channel_send", bitcode::encode(&send)),
        ("remote_send", bitcode::encode(&remote)),
        ("remote_row", bitcode::encode(&row)),
        ("spam_thresholds", bitcode::encode(&thresholds)),
        ("artifact_ref", bitcode::encode(&reference)),
        ("purview_row", bitcode::encode(&PurviewRow { id: "core".into(), namepath_patterns: vec!["gram:".into()] })),
    ]
}

/// Every vector's name and its encoding.
fn encoded() -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for (name, e) in operator() {
        out.insert(name, bitcode::encode(&e));
    }
    for (name, e) in runtime_errors() {
        out.insert(format!("runtime_error/{name}"), bitcode::encode(&e));
    }
    for (name, e) in transport_errors() {
        out.insert(format!("transport_error/{name}"), bitcode::encode(&e));
    }
    for (name, r) in requests() {
        out.insert(format!("request/{name}"), bitcode::encode(&r));
    }
    for (name, r) in replies() {
        out.insert(format!("reply/{name}"), bitcode::encode(&r));
    }
    for (name, n) in channel() {
        out.insert(format!("channel_notice/{name}"), bitcode::encode(&n));
    }
    for (name, e) in peer() {
        out.insert(format!("remote_event/{name}"), bitcode::encode(&e));
    }
    for (name, request, reply) in artifacts() {
        out.insert(format!("artifact_request/{name}"), bitcode::encode(&request));
        out.insert(format!("artifact_reply/{name}"), bitcode::encode(&reply));
    }
    for (name, bytes) in misc() {
        out.insert(format!("data/{name}"), bytes);
    }
    out
}

#[test]
fn every_vector_encodes_to_its_golden_bytes() {
    let actual = golden::hex_table(encoded());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an encoding moved; a deliberate change re-blesses {GOLDEN}");
}

/// A value's bitcode round trip, compared whole.
fn round_trips<T>(name: &str, value: &T)
where
    T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
{
    let back: T = decode(&bitcode::encode(value)).expect(name);
    assert_eq!(&back, value, "{name}");
}

#[test]
fn every_vector_round_trips() {
    for (name, e) in operator() {
        round_trips(&name, &e);
    }
    for (name, e) in runtime_errors() {
        round_trips(name, &e);
    }
    for (name, e) in transport_errors() {
        round_trips(name, &e);
    }
    for (name, r) in requests() {
        round_trips(name, &r);
    }
    for (name, r) in replies() {
        round_trips(name, &r);
    }
    for (name, n) in channel() {
        round_trips(name, &n);
    }
    for (name, e) in peer() {
        round_trips(name, &e);
    }
    for (name, request, reply) in artifacts() {
        round_trips(name, &request);
        round_trips(name, &reply);
    }
}

#[test]
fn every_variant_has_its_vector() {
    let emergencies: BTreeSet<_> = operator().iter().map(|(_, e)| e.kind().name()).collect();
    assert_eq!(emergencies.len(), 9);
    let requests: BTreeSet<_> = requests()
        .iter()
        .map(|(_, r)| match r {
            RuntimeRequest::Engine(_) => "engine",
            RuntimeRequest::Interface { .. } => "interface",
            RuntimeRequest::Info => "info",
            RuntimeRequest::OperationLookup { .. } => "operation_lookup",
            RuntimeRequest::Policy(_) => "policy",
            RuntimeRequest::Channel(_) => "channel",
            RuntimeRequest::Artifact(_) => "artifact",
            RuntimeRequest::Cancel { .. } => "cancel",
        })
        .collect();
    assert_eq!(requests.len(), 8);
    let replies: BTreeSet<_> = replies()
        .iter()
        .map(|(_, r)| match r {
            RuntimeReply::Engine(_) => "engine",
            RuntimeReply::Interface { .. } => "interface",
            RuntimeReply::Info(_) => "info",
            RuntimeReply::Operation(_) => "operation",
            RuntimeReply::Policy(_) => "policy",
            RuntimeReply::Channel(_) => "channel",
            RuntimeReply::Artifact(_) => "artifact",
            RuntimeReply::Refused(_) => "refused",
        })
        .collect();
    assert_eq!(replies.len(), 8);
    let notices: BTreeSet<_> = channel()
        .iter()
        .map(|(_, n)| match n {
            ChannelNotice::Opened { .. } => "opened",
            ChannelNotice::Claimed { .. } => "claimed",
            ChannelNotice::Ready { .. } => "ready",
            ChannelNotice::Closed { .. } => "closed",
        })
        .collect();
    assert_eq!(notices.len(), 4);
    let events: BTreeSet<_> = peer().iter().map(|(_, e)| e.model()).collect();
    assert_eq!(events.len(), 4);
    let artifact_requests: BTreeSet<_> = artifacts()
        .iter()
        .map(|(_, r, _)| match r {
            ArtifactRequest::Reserve(_) => "reserve",
            ArtifactRequest::Publish(_) => "publish",
            ArtifactRequest::Inspect(_) => "inspect",
            ArtifactRequest::Abort(_) => "abort",
            ArtifactRequest::Describe(_) => "describe",
            ArtifactRequest::ReadLog(_) => "read_log",
        })
        .collect();
    assert_eq!(artifact_requests.len(), 6);
    let kinds: BTreeSet<_> = runtime_errors()
        .into_iter()
        .filter(|(_, e)| !matches!(e, RuntimeError::InterfaceViolations { .. }))
        .map(|(_, e)| e.kind())
        .collect();
    assert_eq!(kinds.len(), 16);
    let transport: BTreeSet<_> = transport_errors().iter().map(|(_, e)| e.kind()).collect();
    assert_eq!(transport.len(), 16);
}

#[test]
fn a_violations_refusal_reports_its_errors_alone() {
    let rows = RuntimeError::InterfaceViolations { diagnostics: vec![refusal(), warning()] }.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "interface::invalid");
    let single = RuntimeError::ChannelWrongMode { open: ChannelMode::Async, asked: ChannelMode::Sync }.rows();
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].kind, "channel::wrong_mode");
    assert!(single[0].message.contains("open in `async` mode, not `sync`"), "{}", single[0].message);
}

#[test]
fn a_transport_failure_keeps_authentication_apart_from_an_unknown_outcome() {
    for (name, error) in transport_errors() {
        assert!(error.kind().starts_with("daemon::"), "{name}");
        assert_eq!(error.clone().rows().len(), 1, "{name}");
    }
    let auth = TransportError::AuthenticationFailed { reason: "publickey".into() };
    let unknown = TransportError::UnknownOutcome { operation: OPERATION, reason: "eof".into() };
    assert_ne!(auth.kind(), unknown.kind());
    assert!(unknown.message().contains(&OPERATION.hex()), "{}", unknown.message());
    assert!(auth.message().contains("before any request was sent"));
}

#[test]
fn a_mode_and_a_link_state_serialize_as_their_words() {
    let word = |s: &str| NuVal::String(s.into());
    assert_eq!(nuval::to_nuval(&ChannelMode::Async).expect("a value"), word("async"));
    assert_eq!(nuval::to_nuval(&RemoteLinkState::Listening).expect("a value"), word("listening"));
    assert_eq!(
        nuval::to_nuval(&PluginInfo("p".into(), None)).expect("a value"),
        NuVal::List(vec![word("p"), NuVal::Nothing]),
    );
    assert_eq!(
        nuval::to_nuval(&PurviewView("core".into(), vec![])).expect("a value"),
        NuVal::List(vec![word("core"), NuVal::List(vec![])]),
    );
}

#[test]
fn the_reservations_and_caps_hold_their_values() {
    assert_eq!(reserved_prefix("mcp/channel/Open"), Some(MCP_RESERVED_PREFIX));
    assert_eq!(reserved_prefix("remote/channel/Sent"), Some(REMOTE_RESERVED_PREFIX));
    assert_eq!(reserved_prefix("example/Note"), None);
    assert_eq!(SYNC_QUEUE_CAP, 1000);
    assert_eq!(LOCAL_HOST, "local");
}

const _: () = assert!(MAX_FRAME_BYTES < 1 << 20);
const _: () = assert!(LOG_READ_MAX <= SFTP_CHUNK_MAX && SFTP_CHUNK_MAX < SFTP_PACKET_MAX);
