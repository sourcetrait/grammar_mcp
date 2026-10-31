//! The `wire` group: the frozen v1 vectors held to this build's protocol.
use crate::*;
use sourcetrait_grammar_protocol::fixture::{self, FixtureVector};
use sourcetrait_grammar_protocol::{
    ARTIFACT_LIMITS_DEFAULT, AdmissionOutcome, CODEC_ID, ClientHello, DECODE_BUDGET_DEFAULT, EVENT_FRAME_MAX, Fresh,
    HELLO_BUDGET, HELLO_FRAME_MAX, HelloReply, HelloState, LIMITS_DEFAULT, LeaseId, LeaseTable, LeaseToken,
    OperationTable, PROTOCOL_VERSION, ProtocolVersion, RPC_FRAME_DEFAULT, SCHEMA_ID, Submission, WorkerGeneration,
    WorkerProfile, check_hello, decode_payload,
};

/// The vector files the manifest pins, as this probe was built with them.
const FILES: [(&str, &str); 10] = [
    ("artifact.nuon", include_str!("../../../../crate/protocol/tests/vectors/artifact.nuon")),
    ("engine.nuon", include_str!("../../../../crate/protocol/tests/vectors/engine.nuon")),
    ("event.nuon", include_str!("../../../../crate/protocol/tests/vectors/event.nuon")),
    ("hello.nuon", include_str!("../../../../crate/protocol/tests/vectors/hello.nuon")),
    ("lease.nuon", include_str!("../../../../crate/protocol/tests/vectors/lease.nuon")),
    ("operation.nuon", include_str!("../../../../crate/protocol/tests/vectors/operation.nuon")),
    ("peer.nuon", include_str!("../../../../crate/protocol/tests/vectors/peer.nuon")),
    ("runtime.nuon", include_str!("../../../../crate/protocol/tests/vectors/runtime.nuon")),
    ("stream.nuon", include_str!("../../../../crate/protocol/tests/vectors/stream.nuon")),
    ("value.nuon", include_str!("../../../../crate/protocol/tests/vectors/value.nuon")),
];

/// The manifest pinning them, the schema, and the codec.
const MANIFEST: &str = include_str!("../../../../crate/protocol/tests/vectors/manifest.nuon");

/// The group's assertions, in the order they run.
pub(crate) const ASSERTIONS: [&str; 6] =
    ["wire.manifest", "wire.vectors", "wire.types", "wire.limits", "wire.handshake", "wire.execution"];

/// The embedded files, each with its vectors.
struct Fixture {
    files: Vec<(&'static str, &'static str, Vec<FixtureVector>)>,
}

impl Fixture {
    /// Every vector, of every file.
    fn vectors(&self) -> impl Iterator<Item = &FixtureVector> {
        self.files.iter().flat_map(|(_, _, vectors)| vectors)
    }
}

/// The group run: every assertion passed, or the index of the one that failed.
pub(crate) fn run() -> Result<(), (usize, String)> {
    let files = FILES
        .iter()
        .map(|(name, text)| fixture::read_vectors(text).map(|vectors| (*name, *text, vectors)).map_err(|e| (0, format!("{name}: {e}"))))
        .collect::<Result<Vec<_>, _>>()?;
    let fixture = Fixture { files };
    let checks: [fn(&Fixture) -> Result<(), String>; 6] = [manifest, vectors, types, limits, handshake, execution];
    for (at, check) in checks.iter().enumerate() {
        check(&fixture).map_err(|reason| (at, format!("{}: {reason}", ASSERTIONS[at])))?;
    }
    Ok(())
}

/// Bytes in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A record's field.
fn field<'a>(fields: &'a [(String, NuVal)], key: &str) -> Result<&'a NuVal, String> {
    fields.iter().find(|(name, _)| name == key).map(|(_, value)| value).ok_or_else(|| format!("no `{key}`"))
}

/// A record's text field.
fn text<'a>(fields: &'a [(String, NuVal)], key: &str) -> Result<&'a str, String> {
    match field(fields, key)? {
        NuVal::String(text) => Ok(text),
        _ => Err(format!("`{key}` isn't text")),
    }
}

/// The manifest names this build's version, schema, and codec, and pins each file.
fn manifest(fixture: &Fixture) -> Result<(), String> {
    let NuVal::Record(fields) = NuVal::from_nuon(MANIFEST).map_err(|e| e.to_string())? else {
        return Err("a manifest that isn't a record".to_string());
    };
    let version = format!("{}.{}", PROTOCOL_VERSION.major, PROTOCOL_VERSION.minor);
    for (key, want) in [("version", version), ("schema_id", hex(&SCHEMA_ID)), ("codec", CODEC_ID.to_string())] {
        let held = text(&fields, key)?;
        if held != want {
            return Err(format!("its {key} is {held}, this build's {want}"));
        }
    }
    let NuVal::List(rows) = field(&fields, "files")? else { return Err("files that aren't a table".to_string()) };
    if rows.len() != fixture.files.len() {
        return Err(format!("{} files pinned, {} embedded", rows.len(), fixture.files.len()));
    }
    for (name, file_text, vectors) in &fixture.files {
        let row = rows
            .iter()
            .find_map(|row| match row {
                NuVal::Record(row) if text(row, "file").is_ok_and(|file| file == *name) => Some(row),
                _ => None,
            })
            .ok_or_else(|| format!("{name} isn't pinned"))?;
        let digest = hex(&fixture::digest(file_text).map_err(|e| format!("{name}: {e}"))?);
        if text(row, "sha256")? != digest {
            return Err(format!("{name}'s digest isn't the pinned one"));
        }
        if *field(row, "vectors")? != NuVal::Int(vectors.len() as i64) {
            return Err(format!("{name} holds {} vectors, not the pinned count", vectors.len()));
        }
    }
    Ok(())
}

/// Every vector decodes by its type to its own bytes, and refuses malformed forms.
fn vectors(fixture: &Fixture) -> Result<(), String> {
    fixture.vectors().try_for_each(|vector| fixture::check_vector(vector, DECODE_BUDGET_DEFAULT))
}

/// Every driver type names a vector, and every vector's type a driver.
fn types(fixture: &Fixture) -> Result<(), String> {
    let named: std::collections::BTreeSet<&str> = fixture.vectors().map(|v| v.ty.as_str()).collect();
    let known: std::collections::BTreeSet<&str> = fixture::FIXTURE_TYPES.iter().copied().collect();
    match named.symmetric_difference(&known).next() {
        Some(ty) => Err(format!("`{ty}` is a vector's type or a driver's, not both")),
        None => Ok(()),
    }
}

/// Every vector fits its stream's frame, a hello within its own budget too.
fn limits(fixture: &Fixture) -> Result<(), String> {
    for vector in fixture.vectors() {
        let (ceiling, hello) = match vector.ty.as_str() {
            "ClientHello" | "HelloReply" => (HELLO_FRAME_MAX, true),
            "EventFrame" => (EVENT_FRAME_MAX, false),
            _ => (RPC_FRAME_DEFAULT, false),
        };
        if vector.bytes.len() > ceiling as usize {
            return Err(format!("{} passes its {ceiling}-byte frame", vector.name));
        }
        if hello && !matches!(fixture::canonical(&vector.ty, &vector.bytes, HELLO_BUDGET), Some(Ok(_))) {
            return Err(format!("{} passes the hello's budget", vector.name));
        }
    }
    Ok(())
}

/// A hello of another major, schema, or codec is refused, nothing bound.
fn handshake(fixture: &Fixture) -> Result<(), String> {
    let vector = fixture.vectors().find(|v| v.ty == "ClientHello").ok_or("no client hello")?;
    let hello: ClientHello = decode_payload(&vector.bytes, HELLO_BUDGET).map_err(|e| e.to_string())?;
    let generation = WorkerGeneration([0x44; 16]);
    let worker = WorkerProfile {
        uid: 0,
        instance: hello.expected.clone(),
        generation,
        limits: LIMITS_DEFAULT,
        artifacts: ARTIFACT_LIMITS_DEFAULT,
        capabilities: sourcetrait_grammar_protocol::Capabilities { wire: true, ..sourcetrait_grammar_protocol::Capabilities::NONE },
    };
    check_hello(&hello, &worker).map_err(|e| format!("the frozen hello refused: {}", e.message()))?;
    let fresh = Fresh { lease: LeaseId([0x66; 16]), token: LeaseToken([0x77; 32]) };
    let altered = [
        ("daemon::version_mismatch", ClientHello { version: ProtocolVersion { major: PROTOCOL_VERSION.major + 1, minor: 0 }, ..hello.clone() }),
        ("daemon::schema_mismatch", ClientHello { schema: [0xff; 32], ..hello.clone() }),
        ("daemon::codec_mismatch", ClientHello { codec: "bitcode-0.6.8".to_string(), ..hello.clone() }),
    ];
    for (kind, hello) in altered {
        let mut state = HelloState::default();
        let mut leases = LeaseTable::new(generation);
        let (reply, actions) = state.on_hello(&hello, &worker, &mut leases, 0, fresh);
        match reply {
            HelloReply::Refused(e) if e.kind() == kind => {}
            other => return Err(format!("a hello wanting {kind} answered {other:?}")),
        }
        if !actions.is_empty() || leases.live(0).is_some() || state.admit().is_ok() {
            return Err(format!("a hello refused as {kind} bound a lease or admits a request"));
        }
    }
    Ok(())
}

/// No submission, whole, cut short, or mutated, runs without a worker's ticket.
fn execution(fixture: &Fixture) -> Result<(), String> {
    let generation = WorkerGeneration([0x44; 16]);
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for vector in fixture.vectors().filter(|v| v.ty == "Submission") {
        let mut forms: Vec<Vec<u8>> = (0..=vector.bytes.len()).map(|cut| vector.bytes[..cut].to_vec()).collect();
        for _ in 0..256 {
            let mut bytes = vector.bytes.clone();
            let at = (next() as usize) % bytes.len().max(1);
            if let Some(byte) = bytes.get_mut(at) {
                *byte ^= 1 << (next() % 8);
            }
            forms.push(bytes);
        }
        for bytes in forms {
            let Ok(submission) = decode_payload::<Submission>(&bytes, DECODE_BUDGET_DEFAULT) else { continue };
            let (admission, execute) = OperationTable::new(generation).submit(0, &submission, submission.ticket.epoch);
            if execute.is_some() || !matches!(admission.outcome, AdmissionOutcome::Refused(_)) {
                return Err(format!("{} ran, or was admitted, with no ticket issued", vector.name));
            }
        }
    }
    Ok(())
}
