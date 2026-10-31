//! A stream's handshake: each mismatch refused alone with nothing dispatched,
//! limits negotiated down, the client's checks of the reply, a new generation
//! reported, each stream's state, the instance's names and key, and a golden
//! vector of each hello and reply.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::time::Duration;

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/hello.nuon";

const UID: u32 = 4242;
const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const CLIENT: ClientInstanceId = ClientInstanceId([0x55; 16]);
const FRESH: Fresh = Fresh { lease: LeaseId([0x66; 16]), token: LeaseToken([0x77; 32]) };

fn instance() -> Instance {
    Instance::new("fixture", "default").expect("a valid instance")
}

fn worker() -> WorkerProfile {
    WorkerProfile {
        uid: UID,
        instance: instance(),
        generation: GENERATION,
        limits: LIMITS_DEFAULT,
        artifacts: ARTIFACT_LIMITS_DEFAULT,
        capabilities: Capabilities { wire: true, ..Capabilities::NONE },
    }
}

fn expectation() -> Expectation {
    Expectation {
        uid: UID,
        instance: instance(),
        role: StreamRole::Rpc,
        lease: LeaseIntent::Claim,
        requested: LIMITS_DEFAULT,
        required: vec!["wire".to_string()],
        last: None,
    }
}

fn hello() -> ClientHello {
    expectation().hello(CLIENT)
}

/// A fresh worker's reply to `hello`, on a stream of its own.
fn answer(hello: &ClientHello) -> HelloReply {
    let mut leases = LeaseTable::new(GENERATION);
    HelloState::default().on_hello(hello, &worker(), &mut leases, 0, FRESH).0
}

/// The worker's acceptance of the fixture hello.
fn reply() -> ServerHello {
    match answer(&hello()) {
        HelloReply::Accepted(accepted) => *accepted,
        refused => panic!("the fixture hello refused: {refused:?}"),
    }
}

/// A worker's stream counting what it dispatches: only an admitted request is.
struct Stream {
    state: HelloState,
    leases: LeaseTable,
    dispatched: Cell<u32>,
}

impl Stream {
    fn new() -> Self {
        Self { state: HelloState::default(), leases: LeaseTable::new(GENERATION), dispatched: Cell::new(0) }
    }

    fn hello(&mut self, hello: &ClientHello) -> HelloReply {
        self.state.on_hello(hello, &worker(), &mut self.leases, 0, FRESH).0
    }

    fn request(&self) -> Result<(), TransportError> {
        self.state.admit()?;
        self.dispatched.set(self.dispatched.get() + 1);
        Ok(())
    }
}

/// `hello`'s refusal by a fresh stream, nothing dispatched before or after it.
fn refused(hello: &ClientHello) -> TransportError {
    let mut stream = Stream::new();
    assert!(stream.request().is_err(), "a request admitted before the hello");
    let error = match stream.hello(hello) {
        HelloReply::Refused(error) => error,
        accepted => panic!("accepted: {accepted:?}"),
    };
    assert!(stream.request().is_err(), "a request admitted after a refusal");
    assert_eq!(stream.dispatched.get(), 0);
    error
}

#[test]
fn a_valid_hello_readies_the_stream_and_admits_its_requests() {
    let mut stream = Stream::new();
    let HelloReply::Accepted(accepted) = stream.hello(&hello()) else { panic!("refused") };
    assert_eq!(accepted.worker.key, instance().key());
    assert_eq!(accepted.worker.generation, GENERATION);
    assert_eq!(accepted.effective, LIMITS_DEFAULT);
    stream.request().expect("admitted");
    assert_eq!(stream.dispatched.get(), 1);
}

#[test]
fn each_mismatch_is_refused_alone_with_nothing_dispatched() {
    let named = |id: &str, namespace: &str| Instance::new(id, namespace).expect("a valid instance");
    let cases = [
        ("a major version", ClientHello { version: ProtocolVersion { major: 2, minor: 0 }, ..hello() }, "daemon::version_mismatch"),
        ("a schema", ClientHello { schema: [0; 32], ..hello() }, "daemon::schema_mismatch"),
        ("a codec", ClientHello { codec: "bitcode-0.7.0".to_string(), ..hello() }, "daemon::codec_mismatch"),
        ("an id", ClientHello { expected: named("other", "default"), ..hello() }, "daemon::wrong_worker"),
        ("a namespace", ClientHello { expected: named("fixture", "test"), ..hello() }, "daemon::wrong_worker"),
        (
            "a malformed name",
            ClientHello { expected: Instance { id: "../x".to_string(), namespace: "default".to_string() }, ..hello() },
            "daemon::invalid_frame",
        ),
        ("an unknown feature", ClientHello { required: vec!["teleport".to_string()], ..hello() }, "daemon::unsupported_feature"),
        ("a feature off", ClientHello { required: vec!["artifacts".to_string()], ..hello() }, "daemon::unsupported_feature"),
        (
            "too many features",
            ClientHello { required: vec!["wire".to_string(); REQUIRED_FEATURES_MAX + 1], ..hello() },
            "daemon::invalid_frame",
        ),
        (
            "a zero limit",
            ClientHello { requested: Limits { control_slots: 0, ..LIMITS_DEFAULT }, ..hello() },
            "daemon::invalid_frame",
        ),
        ("a Status stream claiming", ClientHello { role: StreamRole::Status, ..hello() }, "daemon::invalid_frame"),
        ("an Events stream claiming", ClientHello { role: StreamRole::Events, ..hello() }, "daemon::invalid_frame"),
        ("an RPC stream observing", ClientHello { lease: LeaseIntent::Observe, ..hello() }, "daemon::invalid_frame"),
    ];
    for (name, hello, kind) in cases {
        let error = refused(&hello);
        assert_eq!(error.kind(), kind, "{name}: {}", error.message());
    }
}

#[test]
fn a_refused_hello_binds_no_lease() {
    let mut stream = Stream::new();
    let bad = ClientHello { schema: [0; 32], ..hello() };
    assert!(matches!(stream.hello(&bad), HelloReply::Refused(_)));
    assert_eq!(stream.leases.live(0), None);
}

#[test]
fn a_minor_version_beneath_one_schema_is_accepted() {
    let newer = ClientHello { version: ProtocolVersion { major: 1, minor: 7 }, ..hello() };
    assert!(check_hello(&newer, &worker()).is_ok());
    let HelloReply::Accepted(accepted) = answer(&newer) else { panic!("refused") };
    assert_eq!(accepted.version, PROTOCOL_VERSION);
}

#[test]
fn raised_limits_negotiate_down_to_the_worker_s_and_the_hard_ceilings() {
    let greedy = Limits {
        rpc_frame: u32::MAX,
        decode_heap: u64::MAX,
        concurrent_rpcs: u32::MAX,
        control_slots: u32::MAX,
        event_records: u32::MAX,
        event_bytes: u64::MAX,
    };
    let asked = ClientHello { requested: greedy, ..hello() };
    assert_eq!(check_hello(&asked, &worker()).expect("accepted"), LIMITS_DEFAULT);
    // A worker configured past the hard ceilings still grants none past them.
    let lavish = WorkerProfile { limits: greedy, ..worker() };
    assert_eq!(check_hello(&asked, &lavish).expect("accepted"), LIMITS_HARD);
    // A modest request is granted as asked.
    let modest = Limits { rpc_frame: 1 << 20, concurrent_rpcs: 2, ..LIMITS_DEFAULT };
    let granted = check_hello(&ClientHello { requested: modest, ..hello() }, &worker()).expect("accepted");
    assert_eq!(granted, modest);
    assert_eq!(granted.frame_ceiling(StreamRole::Rpc), 1 << 20);
    assert_eq!(granted.frame_ceiling(StreamRole::Status), 1 << 20);
    assert_eq!(granted.frame_ceiling(StreamRole::Events), EVENT_FRAME_MAX);
    assert_eq!(granted.decode_budget(), DecodeBudget { heap: modest.decode_heap, ..DECODE_BUDGET_DEFAULT });
}

#[test]
fn the_client_accepts_the_worker_it_expects() {
    let session = check_reply(HelloReply::Accepted(Box::new(reply())), &expectation()).expect("accepted");
    assert_eq!(session.worker.uid, UID);
    assert_eq!(session.worker.instance, instance());
    assert_eq!(session.limits, LIMITS_DEFAULT);
    assert_eq!(session.artifacts, ARTIFACT_LIMITS_DEFAULT);
    assert!(!session.new_generation);
    // A worker's store bounds below the defaults are its own to set.
    let lower = ArtifactLimits { reservations: 2, store_bytes: ARTIFACT_MAX_BYTES, ..ARTIFACT_LIMITS_DEFAULT };
    let session = check_reply(HelloReply::Accepted(Box::new(ServerHello { artifacts: lower, ..reply() })), &expectation())
        .expect("accepted");
    assert_eq!(session.artifacts, lower);
}

#[test]
fn the_client_refuses_each_wrong_reply() {
    let other = Instance::new("fixture", "test").expect("a valid instance");
    let cases = [
        ("a major version", ServerHello { version: ProtocolVersion { major: 0, minor: 9 }, ..reply() }, "daemon::version_mismatch"),
        ("a schema", ServerHello { schema: [1; 32], ..reply() }, "daemon::schema_mismatch"),
        ("a codec", ServerHello { codec: "bitcode-0.6.8".to_string(), ..reply() }, "daemon::codec_mismatch"),
        ("a uid", ServerHello { worker: WorkerIdentity { uid: UID + 1, ..reply().worker }, ..reply() }, "daemon::wrong_worker"),
        (
            "an instance",
            ServerHello { worker: WorkerIdentity { key: other.key(), instance: other.clone(), ..reply().worker }, ..reply() },
            "daemon::wrong_worker",
        ),
        (
            "a forged key",
            ServerHello { worker: WorkerIdentity { key: InstanceKey([7; 32]), ..reply().worker }, ..reply() },
            "daemon::invalid_frame",
        ),
        ("a role", ServerHello { role: StreamRole::Status, ..reply() }, "daemon::invalid_frame"),
        (
            "a raised limit",
            ServerHello { effective: Limits { rpc_frame: RPC_FRAME_HARD, ..LIMITS_DEFAULT }, ..reply() },
            "daemon::invalid_frame",
        ),
        (
            "a zero limit",
            ServerHello { effective: Limits { event_records: 0, ..LIMITS_DEFAULT }, ..reply() },
            "daemon::invalid_frame",
        ),
        ("a claim left unbound", ServerHello { lease: LeaseBinding::Unbound, ..reply() }, "daemon::invalid_frame"),
        ("a feature off", ServerHello { capabilities: Capabilities::NONE, ..reply() }, "daemon::unsupported_feature"),
        (
            "an artifact bound past its ceiling",
            ServerHello { artifacts: ArtifactLimits { reservations: 9, ..ARTIFACT_LIMITS_DEFAULT }, ..reply() },
            "daemon::invalid_frame",
        ),
        (
            "a retention short of the result window",
            ServerHello { artifacts: ArtifactLimits { retention_secs: 3600, ..ARTIFACT_LIMITS_DEFAULT }, ..reply() },
            "daemon::invalid_frame",
        ),
    ];
    for (name, hello, kind) in cases {
        let error = check_reply(HelloReply::Accepted(Box::new(hello)), &expectation()).expect_err(name);
        assert_eq!(error.kind(), kind, "{name}: {}", error.message());
    }
    // A refusal is the worker's own error, passed through.
    let refusal = HelloReply::Refused(TransportError::UnsupportedFeature { feature: "artifacts".to_string() });
    assert_eq!(check_reply(refusal, &expectation()).expect_err("refused").kind(), "daemon::unsupported_feature");
}

#[test]
fn the_client_holds_each_limit_to_the_hard_ceilings_whatever_it_asked() {
    let greedy = Limits {
        rpc_frame: u32::MAX,
        decode_heap: u64::MAX,
        concurrent_rpcs: u32::MAX,
        control_slots: u32::MAX,
        event_records: u32::MAX,
        event_bytes: u64::MAX,
    };
    let expect = Expectation { requested: greedy, ..expectation() };
    let past = [
        ("rpc_frame", Limits { rpc_frame: LIMITS_HARD.rpc_frame + 1, ..LIMITS_HARD }),
        ("decode_heap", Limits { decode_heap: LIMITS_HARD.decode_heap + 1, ..LIMITS_HARD }),
        ("concurrent_rpcs", Limits { concurrent_rpcs: LIMITS_HARD.concurrent_rpcs + 1, ..LIMITS_HARD }),
        ("control_slots", Limits { control_slots: LIMITS_HARD.control_slots + 1, ..LIMITS_HARD }),
        ("event_records", Limits { event_records: LIMITS_HARD.event_records + 1, ..LIMITS_HARD }),
        ("event_bytes", Limits { event_bytes: LIMITS_HARD.event_bytes + 1, ..LIMITS_HARD }),
    ];
    for (name, effective) in past {
        let error = check_reply(HelloReply::Accepted(Box::new(ServerHello { effective, ..reply() })), &expect).expect_err(name);
        assert_eq!(error.kind(), "daemon::invalid_frame", "{name}");
        assert!(error.message().contains(name), "{name}: {}", error.message());
    }
    let at_the_ceilings = ServerHello { effective: LIMITS_HARD, ..reply() };
    assert_eq!(check_reply(HelloReply::Accepted(Box::new(at_the_ceilings)), &expect).expect("accepted").limits, LIMITS_HARD);
}

#[test]
fn a_resume_or_an_attachment_is_refused_from_another_generation() {
    let grant = LeaseGrant { lease: LeaseId([1; 16]), token: LeaseToken([2; 32]), epoch: LeaseEpoch(1), heartbeat_ms: 10_000, expiry_ms: 30_000 };
    let earlier = WorkerGeneration([0x43; 16]);
    let cases = [
        (StreamRole::Rpc, LeaseIntent::Resume { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation: earlier }, LeaseBinding::Granted(grant.clone())),
        (StreamRole::Events, LeaseIntent::Attach { lease: grant.lease, token: grant.token, epoch: grant.epoch, generation: earlier }, LeaseBinding::Attached { lease: grant.lease, epoch: grant.epoch }),
    ];
    for (role, lease, binding) in cases {
        let name = lease.name();
        // The reply's worker is GENERATION's, not the intent's.
        let reply = ServerHello { role, lease: binding, ..reply() };
        for last in [None, Some(earlier)] {
            let expect = Expectation { role, lease: lease.clone(), last, ..expectation() };
            let error = check_reply(HelloReply::Accepted(Box::new(reply.clone())), &expect).expect_err(name);
            assert_eq!(error.kind(), "daemon::invalid_frame", "{name} with last {last:?}");
        }
    }
}

#[test]
fn a_new_generation_is_reported_so_nothing_of_the_old_worker_carries_over() {
    let same = Expectation { last: Some(GENERATION), ..expectation() };
    assert!(!check_reply(HelloReply::Accepted(Box::new(reply())), &same).expect("accepted").new_generation);
    let restarted = Expectation { last: Some(WorkerGeneration([0x45; 16])), ..expectation() };
    assert!(check_reply(HelloReply::Accepted(Box::new(reply())), &restarted).expect("accepted").new_generation);
}

#[test]
fn a_second_hello_closes_the_stream_and_a_fresh_stream_awaits_its_own() {
    let mut stream = Stream::new();
    assert!(matches!(stream.hello(&hello()), HelloReply::Accepted(_)));
    stream.request().expect("admitted");
    // A second hello, the one way a role could change, is refused and closes the stream.
    let events = ClientHello { role: StreamRole::Events, ..hello() };
    let HelloReply::Refused(error) = stream.hello(&events) else { panic!("a second hello accepted") };
    assert_eq!(error.kind(), "daemon::invalid_frame");
    assert!(stream.request().is_err(), "a request admitted after the stream closed");
    assert!(matches!(stream.hello(&hello()), HelloReply::Refused(_)), "a closed stream reopened");
    assert_eq!(stream.dispatched.get(), 1);
    // A reconnect is a new stream, awaiting its own hello.
    assert!(Stream::new().request().is_err());
}

#[test]
fn both_ends_agree_on_one_set_of_fixtures() {
    let mut leases = LeaseTable::new(GENERATION);
    let mut run = |expect: Expectation| {
        let asked = expect.hello(CLIENT);
        let mut state = HelloState::default();
        let (reply, _) = state.on_hello(&asked, &worker(), &mut leases, 0, FRESH);
        let session = check_reply(reply, &expect).expect("both ends accept");
        assert_eq!(state.admit().expect("ready").effective, session.limits);
        assert_eq!(session.role, asked.role);
        session
    };
    let LeaseBinding::Granted(grant) = run(expectation()).lease else { panic!("a claim left ungranted") };
    let held = |intent: fn(LeaseId, LeaseToken, LeaseEpoch, WorkerGeneration) -> LeaseIntent| {
        intent(grant.lease, grant.token, grant.epoch, GENERATION)
    };
    let attach = held(|lease, token, epoch, generation| LeaseIntent::Attach { lease, token, epoch, generation });
    let resume = held(|lease, token, epoch, generation| LeaseIntent::Resume { lease, token, epoch, generation });
    let events = run(Expectation { role: StreamRole::Events, lease: attach, ..expectation() });
    assert_eq!(events.lease, LeaseBinding::Attached { lease: grant.lease, epoch: grant.epoch });
    let status = run(Expectation { role: StreamRole::Status, lease: LeaseIntent::Observe, required: Vec::new(), ..expectation() });
    assert_eq!(status.lease, LeaseBinding::Unbound);
    let modest = Limits { rpc_frame: 1 << 16, ..LIMITS_DEFAULT };
    let resumed = run(Expectation { lease: resume, requested: modest, ..expectation() });
    assert_eq!(resumed.lease, LeaseBinding::Granted(grant));
}

#[test]
fn the_deadlines_are_finite() {
    assert_eq!(HELLO_DEADLINE, Duration::from_secs(5));
    assert_eq!(AUTH_DEADLINE, Duration::from_secs(30));
}

#[test]
fn the_instance_key_hashes_each_name_s_length_and_bytes() {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::new();
    for name in ["fixture", "default"] {
        bytes.extend((name.len() as u32).to_be_bytes());
        bytes.extend(name.as_bytes());
    }
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    assert_eq!(instance().key(), InstanceKey(digest));
    assert_eq!(instance().key().hex().len(), 64);
    // The lengths keep the names apart.
    let split = |id: &str, namespace: &str| Instance::new(id, namespace).expect("a valid instance").key();
    assert_ne!(split("ab", "c"), split("a", "bc"));
}

#[test]
fn an_instance_name_is_refused_unless_valid_and_never_sanitized() {
    let long = "x".repeat(65);
    for bad in ["", ".", "..", "-x", "_x", ".x", "a/b", "a b", "a\0b", "\u{e9}", long.as_str()] {
        assert!(Instance::new(bad, "default").is_err(), "{bad:?} as an id");
        assert!(Instance::new("fixture", bad).is_err(), "{bad:?} as a namespace");
    }
    let longest = "x".repeat(64);
    for good in ["a", "A9", "a.b_c-d", longest.as_str()] {
        let named = Instance::new(good, good).expect("a valid instance");
        assert_eq!(named.id, good, "sanitized");
    }
}

/// Every hello and reply variant's bitcode.
fn vectors() -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    out.insert("client_hello".to_string(), bitcode::encode(&hello()));
    out.insert("reply/accepted".to_string(), bitcode::encode(&HelloReply::Accepted(Box::new(reply()))));
    let refusal = TransportError::WrongWorker { expected: "fixture/default".to_string(), found: "fixture/test".to_string() };
    out.insert("reply/refused".to_string(), bitcode::encode(&HelloReply::Refused(refusal)));
    out
}

#[test]
fn each_hello_and_reply_has_its_golden_vector_and_decodes_within_the_hello_budget() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "a hello's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let vectors = vectors();
    for (name, bytes) in &vectors {
        assert!(bytes.len() <= HELLO_FRAME_MAX as usize, "{name} fits the hello's frame");
    }
    let client: ClientHello = decode_payload(&vectors["client_hello"], HELLO_BUDGET).expect("decoded");
    assert_eq!(client, hello());
    let accepted: HelloReply = decode_payload(&vectors["reply/accepted"], HELLO_BUDGET).expect("decoded");
    assert_eq!(accepted, HelloReply::Accepted(Box::new(reply())));
    let refused: HelloReply = decode_payload(&vectors["reply/refused"], HELLO_BUDGET).expect("decoded");
    assert!(matches!(refused, HelloReply::Refused(TransportError::WrongWorker { .. })));
    // A hello requiring more names than its budget's elements is refused as it decodes.
    let flood = ClientHello { required: vec![String::new(); 100], ..hello() };
    let outcome = decode_payload::<ClientHello>(&bitcode::encode(&flood), HELLO_BUDGET);
    assert!(matches!(outcome, Err(FrameError::Budget { bound: bitcode::Exceeded::Elements, .. })), "{outcome:?}");
}
