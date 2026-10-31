//! The artifact contract: metadata-only reservations held to their bounds, a
//! publication that is success only once its copy verifies, inspection after an
//! uncertain acknowledgement, a lease's ownership of its reservations, retention
//! and holds before collection, bounded log reads, paths no metadata selects,
//! portable destinations, recovery's adoption and quarantine, and golden vectors.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/artifact.nuon";

const LEASE: LeaseId = LeaseId([0x0a; 16]);
const OTHER: LeaseId = LeaseId([0x0b; 16]);
const GENERATION: WorkerGeneration = WorkerGeneration([0x44; 16]);
const OPERATION: OperationId = OperationId([0x11; 16]);

/// A wall clock's moment, in milliseconds since the epoch.
const NOW: u64 = 1_800_000_000_000;
const HOUR: u64 = 3_600_000;
const WEEK: u64 = 7 * 24 * HOUR;

/// The store's root, the worker's durable state directory.
const ROOT: &str = "/srv/grammard/state";

fn id(n: u8) -> ArtifactId {
    ArtifactId([n; 16])
}

fn remote(path: &str) -> RemotePath {
    RemotePath::new(path).expect("a remote path")
}

fn store() -> ArtifactStore {
    ArtifactStore::new(remote(ROOT), ARTIFACT_LIMITS_DEFAULT)
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// An equip's upload of `size` bytes under a fixture digest.
fn upload(size: u64) -> ReserveUpload {
    ReserveUpload { purpose: ArtifactPurpose::EquipSource, display_name: "geo.tar".to_string(), size, sha256: [0x33; 32] }
}

/// Where the store keeps artifact `n` in `area`.
fn place(area: &str, n: u8) -> String {
    format!("{ROOT}/{area}/{}", id(n).hex())
}

/// The copy a plan declares, measured as declared.
fn copied(plan: &PublishPlan) -> PublishOutcome {
    PublishOutcome::Copied { size: plan.size, sha256: plan.sha256 }
}

/// Artifact `n` reserved, copied, and published under `LEASE` at `NOW`.
fn published(store: &mut ArtifactStore, n: u8, size: u64) -> ArtifactDescription {
    store.reserve(NOW, LEASE, id(n), &upload(size)).expect("reserved");
    let PublishStart::Copy(plan) = store.begin_publish(NOW, LEASE, id(n)).expect("begun") else {
        panic!("a copy to make")
    };
    store.finish_publish(NOW, id(n), copied(&plan)).0.expect("published")
}

/// Artifact `n`'s state at `at`, none where it is unknown.
fn state(store: &ArtifactStore, at: u64, n: u8) -> Option<ArtifactState> {
    match store.inspect(at, id(n)) {
        ArtifactStatus::Known { state, .. } => Some(state),
        ArtifactStatus::Unknown { .. } => None,
    }
}

fn refused(n: u8, state: ArtifactState) -> ArtifactError {
    ArtifactError::State { id: id(n), state }
}

#[test]
fn a_reservation_names_its_staging_path_and_expiry() {
    let mut store = store();
    let reservation = store.reserve(NOW, LEASE, id(1), &upload(4096)).expect("reserved");
    assert_eq!(reservation.id, id(1));
    assert_eq!(reservation.staging_path.as_str(), place("staging", 1));
    assert_eq!(reservation.expires_ms, 3_600_000);
    assert_eq!(state(&store, NOW, 1), Some(ArtifactState::Reserved));
    // No bulk byte rides the RPC: the largest artifact's reservation is a small frame.
    let request = RuntimeRequest::Artifact(ArtifactRequest::Reserve(upload(ARTIFACT_MAX_BYTES)));
    assert!(bitcode::encode(&request).len() < 128);
}

#[test]
fn metadata_selects_no_path_uid_or_namespace() {
    let names = [
        "../../../etc/passwd",
        "/home/other/.ssh/authorized_keys",
        "C:\\Windows\\System32\\drivers",
        "ns:other/namespace",
        "~other/x",
        "uid=0",
    ];
    let mut store = store();
    for (n, name) in (1u8..).zip(names) {
        let request = ReserveUpload { display_name: name.to_string(), ..upload(16) };
        let reservation = store.reserve(NOW, LEASE, id(n), &request).expect(name);
        assert_eq!(reservation.staging_path.as_str(), place("staging", n), "{name}");
        let PublishStart::Copy(plan) = store.begin_publish(NOW, LEASE, id(n)).expect(name) else {
            panic!("{name}: a copy to make")
        };
        assert_eq!((plan.staging.as_str(), plan.object.as_str()), (place("staging", n).as_str(), place("objects", n).as_str()));
        let description = store.finish_publish(NOW, id(n), copied(&plan)).0.expect(name);
        assert_eq!(description.sftp_path.as_str(), place("objects", n), "{name}");
        assert_eq!(description.display_name, name, "a name is metadata, kept verbatim");
    }
}

#[test]
fn invalid_metadata_is_refused_before_anything_is_held() {
    let cases = [
        ("a zero digest", ReserveUpload { sha256: [0; 32], ..upload(16) }, "artifact::invalid"),
        ("an empty artifact's other digest", upload(0), "artifact::invalid"),
        ("an empty name", ReserveUpload { display_name: String::new(), ..upload(16) }, "artifact::invalid"),
        ("a name past its bytes", ReserveUpload { display_name: "n".repeat(DISPLAY_NAME_MAX + 1), ..upload(16) }, "artifact::invalid"),
        ("a control character", ReserveUpload { display_name: "red\u{1b}[31m".to_string(), ..upload(16) }, "artifact::invalid"),
        ("a worker's purpose", ReserveUpload { purpose: ArtifactPurpose::OperationResult, ..upload(16) }, "artifact::invalid"),
        ("a size past the artifact's", upload(ARTIFACT_MAX_BYTES + 1), "artifact::quota"),
    ];
    let mut store = store();
    for (n, (name, request, kind)) in (1u8..).zip(cases) {
        let error = store.reserve(NOW, LEASE, id(n), &request).expect_err(name);
        assert_eq!(error.kind(), kind, "{name}: {}", error.message());
        assert_eq!(state(&store, NOW, n), None, "{name}: nothing held");
    }
    assert_eq!(
        store.reserve(NOW, LEASE, id(20), &upload(ARTIFACT_MAX_BYTES + 1)),
        Err(ArtifactError::Quota { bound: QuotaBound::Artifact, limit: ARTIFACT_MAX_BYTES, requested: ARTIFACT_MAX_BYTES + 1 })
    );
    // The empty digest is an empty artifact's one digest, and a name at its bound fits.
    assert_eq!(EMPTY_SHA256, digest(b""));
    store.reserve(NOW, LEASE, id(21), &ReserveUpload { sha256: EMPTY_SHA256, ..upload(0) }).expect("an empty artifact");
    let at_bound = ReserveUpload { display_name: "n".repeat(DISPLAY_NAME_MAX), ..upload(16) };
    store.reserve(NOW, LEASE, id(22), &at_bound).expect("a name at its bound");
    let held = store.reserve(NOW, LEASE, id(22), &upload(16)).expect_err("an id held");
    assert_eq!(held.kind(), "artifact::invalid");
}

#[test]
fn the_store_holds_its_reservations_and_bytes() {
    let mut store = store();
    for n in 1..=8 {
        store.reserve(NOW, LEASE, id(n), &upload(16)).expect("within the eight");
    }
    assert_eq!(
        store.reserve(NOW, LEASE, id(9), &upload(16)),
        Err(ArtifactError::Quota { bound: QuotaBound::Reservations, limit: 8, requested: 9 })
    );
    // An abort frees its reservation.
    assert_eq!(
        store.abort(NOW, LEASE, id(1)),
        (Ok(id(1)), vec![ArtifactAction::RemoveStaging { path: remote(&place("staging", 1)) }])
    );
    store.reserve(NOW, LEASE, id(9), &upload(16)).expect("room again");
    // The bytes count unfinished reservations and publications together.
    let small = ArtifactLimits { artifact_bytes: 600, store_bytes: 1000, ..ARTIFACT_LIMITS_DEFAULT };
    let mut store = ArtifactStore::new(remote(ROOT), small);
    published(&mut store, 1, 400);
    store.reserve(NOW, LEASE, id(2), &upload(500)).expect("900 of 1000");
    assert_eq!(
        store.reserve(NOW, LEASE, id(3), &upload(200)),
        Err(ArtifactError::Quota { bound: QuotaBound::Store, limit: 1000, requested: 1100 })
    );
    // A lapsed reservation holds nothing.
    store.reserve(NOW + HOUR, LEASE, id(3), &upload(200)).expect("the lapsed 500 freed");
}

#[test]
fn a_publication_is_success_only_once_its_copy_verifies() {
    let mut store = store();
    let bytes = b"an equip's source";
    let request = ReserveUpload { size: bytes.len() as u64, sha256: digest(bytes), ..upload(0) };
    store.reserve(NOW, LEASE, id(1), &request).expect("reserved");
    let begin = |store: &mut ArtifactStore| match store.begin_publish(NOW, LEASE, id(1)).expect("begun") {
        PublishStart::Copy(plan) => plan,
        PublishStart::Done(_) => panic!("not yet published"),
    };
    let plan = begin(&mut store);
    assert_eq!((plan.size, plan.sha256), (request.size, request.sha256));
    // While its copy runs, nothing describes it, aborts it, or begins it again.
    let publishing = refused(1, ArtifactState::Publishing);
    assert_eq!(state(&store, NOW, 1), Some(ArtifactState::Publishing));
    assert_eq!(store.describe(NOW, id(1)), Err(publishing.clone()));
    assert_eq!(store.abort(NOW, LEASE, id(1)), (Err(publishing.clone()), Vec::new()));
    assert_eq!(store.begin_publish(NOW, LEASE, id(1)), Err(publishing));
    // A short copy, a changed one, and a failed one each leave it reserved for a retry.
    let failures = [
        (PublishOutcome::Copied { size: request.size - 1, sha256: digest(&bytes[1..]) }, "artifact::mismatch"),
        (PublishOutcome::Copied { size: request.size, sha256: digest(b"an equip's sourcf") }, "artifact::mismatch"),
        (PublishOutcome::Failed { reason: "no space left on device".to_string() }, "artifact::publish_failed"),
    ];
    for (outcome, kind) in failures {
        let (result, actions) = store.finish_publish(NOW, id(1), outcome);
        assert_eq!(result.expect_err(kind).kind(), kind);
        assert!(actions.is_empty(), "{kind}: the staging file stays for the retry");
        assert_eq!(state(&store, NOW, 1), Some(ArtifactState::Reserved), "{kind}");
        assert_eq!(store.describe(NOW, id(1)), Err(refused(1, ArtifactState::Reserved)), "{kind}");
        begin(&mut store);
    }
    let (result, actions) = store.finish_publish(NOW, id(1), copied(&plan));
    let description = result.expect("published");
    assert_eq!(description.artifact, ArtifactRef { id: id(1), size: request.size, sha256: request.sha256 });
    assert_eq!(actions, vec![ArtifactAction::RemoveStaging { path: plan.staging.clone() }]);
    assert_eq!(store.describe(NOW, id(1)), Ok(description));
    assert_eq!(store.finish_publish(NOW, id(1), copied(&plan)).0, Err(refused(1, ArtifactState::Published)));
}

#[test]
fn an_uncertain_acknowledgement_resolves_by_inspection() {
    let mut store = store();
    let description = published(&mut store, 1, 16);
    // The acknowledgement lost, the client inspects rather than uploading again.
    assert_eq!(store.inspect(NOW, id(1)), ArtifactStatus::Known { id: id(1), state: ArtifactState::Published });
    // A repeat publication answers the same object, whichever lease asks.
    assert_eq!(store.begin_publish(NOW, LEASE, id(1)), Ok(PublishStart::Done(description.clone())));
    assert_eq!(store.begin_publish(NOW, OTHER, id(1)), Ok(PublishStart::Done(description)));
    assert_eq!(store.inspect(NOW, id(9)), ArtifactStatus::Unknown { id: id(9) });
}

#[test]
fn a_reservation_is_its_lease_alone() {
    let mut store = store();
    store.reserve(NOW, LEASE, id(1), &upload(16)).expect("reserved");
    assert_eq!(store.begin_publish(NOW, OTHER, id(1)), Err(ArtifactError::Foreign { id: id(1) }));
    assert_eq!(store.abort(NOW, OTHER, id(1)).0, Err(ArtifactError::Foreign { id: id(1) }));
    assert_eq!(store.begin_publish(NOW, LEASE, id(2)), Err(ArtifactError::Unknown { id: id(2) }));
    assert_eq!(store.abort(NOW, LEASE, id(2)).0, Err(ArtifactError::Unknown { id: id(2) }));
    assert_eq!(store.describe(NOW, id(2)), Err(ArtifactError::Unknown { id: id(2) }));
    // The lease lost, its reservations end with it.
    assert_eq!(store.release(LEASE), vec![ArtifactAction::RemoveStaging { path: remote(&place("staging", 1)) }]);
    assert_eq!(state(&store, NOW, 1), Some(ArtifactState::Aborted));
    assert_eq!(store.begin_publish(NOW, LEASE, id(1)), Err(refused(1, ArtifactState::Aborted)));
    // An abort repeated after a lost acknowledgement answers the same.
    assert_eq!(store.abort(NOW, LEASE, id(1)), (Ok(id(1)), Vec::new()));
}

#[test]
fn a_lost_lease_leaves_a_publication_to_finish_unowned() {
    let mut store = store();
    for n in [1, 2] {
        store.reserve(NOW, LEASE, id(n), &upload(16)).expect("reserved");
        store.begin_publish(NOW, LEASE, id(n)).expect("begun");
    }
    assert!(store.release(LEASE).is_empty(), "a copy under way is not abandoned");
    let (result, _) = store.finish_publish(NOW, id(1), PublishOutcome::Copied { size: 16, sha256: [0x33; 32] });
    assert!(result.is_ok(), "a copy that verifies publishes");
    let (result, actions) = store.finish_publish(NOW, id(2), PublishOutcome::Failed { reason: "fsync failed".to_string() });
    assert_eq!(result.expect_err("failed").kind(), "artifact::publish_failed");
    assert_eq!(actions, vec![ArtifactAction::RemoveStaging { path: remote(&place("staging", 2)) }]);
    assert_eq!(state(&store, NOW, 2), Some(ArtifactState::Aborted), "no lease is left to retry it");
}

#[test]
fn a_lapsed_reservation_is_refused_then_collected() {
    let mut store = store();
    store.reserve(NOW, LEASE, id(1), &upload(16)).expect("reserved");
    let lapsed = NOW + HOUR;
    assert_eq!(state(&store, lapsed - 1, 1), Some(ArtifactState::Reserved));
    assert_eq!(state(&store, lapsed, 1), Some(ArtifactState::Expired));
    assert_eq!(store.begin_publish(lapsed, LEASE, id(1)), Err(refused(1, ArtifactState::Expired)));
    assert_eq!(store.abort(lapsed, LEASE, id(1)).0, Err(refused(1, ArtifactState::Expired)));
    assert_eq!(store.collect(lapsed), vec![ArtifactAction::RemoveStaging { path: remote(&place("staging", 1)) }]);
    assert!(store.collect(lapsed).is_empty(), "collected once");
    assert_eq!(state(&store, lapsed, 1), Some(ArtifactState::Expired));
}

#[test]
fn a_published_artifact_is_never_retracted() {
    let mut store = store();
    published(&mut store, 1, 16);
    assert_eq!(store.abort(NOW, LEASE, id(1)), (Err(refused(1, ArtifactState::Published)), Vec::new()));
    assert!(store.release(LEASE).is_empty());
    assert_eq!(state(&store, NOW, 1), Some(ArtifactState::Published));
}

#[test]
fn retention_and_holds_keep_a_publication_until_both_end() {
    let mut store = store();
    let description = published(&mut store, 1, 16);
    assert!(store.collect(NOW + WEEK - 1).is_empty(), "within its retention");
    store.hold(id(1), ArtifactHold::Result(OPERATION)).expect("held");
    store.hold(id(1), ArtifactHold::Event(7)).expect("held");
    assert!(store.collect(NOW + WEEK).is_empty(), "held past its retention");
    assert!(store.unhold(id(1), ArtifactHold::Result(OPERATION)));
    assert!(!store.unhold(id(1), ArtifactHold::Result(OPERATION)), "let go once");
    assert!(store.collect(NOW + WEEK).is_empty(), "the event still holds it");
    assert!(store.unhold(id(1), ArtifactHold::Event(7)));
    assert_eq!(store.collect(NOW + WEEK), vec![ArtifactAction::RemoveObject { path: description.sftp_path.clone() }]);
    // Its tombstone answers explicitly.
    assert_eq!(state(&store, NOW + WEEK, 1), Some(ArtifactState::Evicted));
    assert_eq!(store.describe(NOW + WEEK, id(1)), Err(refused(1, ArtifactState::Evicted)));
    assert_eq!(store.hold(id(1), ArtifactHold::Reader(1)), Err(refused(1, ArtifactState::Evicted)));
    // A reservation takes no hold.
    store.reserve(NOW, LEASE, id(2), &upload(16)).expect("reserved");
    assert_eq!(store.hold(id(2), ArtifactHold::Reader(1)), Err(refused(2, ArtifactState::Reserved)));
}

#[test]
fn a_worker_publishes_its_own_objects_within_the_store() {
    let small = ArtifactLimits { artifact_bytes: 600, store_bytes: 1000, ..ARTIFACT_LIMITS_DEFAULT };
    let mut store = ArtifactStore::new(remote(ROOT), small);
    let log = ArtifactRef { id: id(1), size: 600, sha256: [0x44; 32] };
    let description = store.publish_local(NOW, log, ArtifactPurpose::OperationLog, "stdout.log").expect("published");
    assert_eq!(description.sftp_path.as_str(), place("objects", 1));
    assert_eq!(state(&store, NOW, 1), Some(ArtifactState::Published));
    let attachment = ArtifactRef { id: id(2), size: 500, sha256: [0x55; 32] };
    assert_eq!(
        store.publish_local(NOW, attachment, ArtifactPurpose::ChannelAttachment, "report.nuon"),
        Err(ArtifactError::Quota { bound: QuotaBound::Store, limit: 1000, requested: 1100 })
    );
    let source = ArtifactRef { id: id(3), size: 10, sha256: [0x66; 32] };
    let upload_only = store.publish_local(NOW, source, ArtifactPurpose::EquipSource, "geo.tar").expect_err("an upload's");
    assert_eq!(upload_only.kind(), "artifact::invalid");
    let again = store.publish_local(NOW, log, ArtifactPurpose::OperationLog, "stdout.log").expect_err("an id held");
    assert_eq!(again.kind(), "artifact::invalid");
}

#[test]
fn a_log_read_is_bounded_and_flags_its_end() {
    let read = |offset, max_bytes| ReadLog { operation: OPERATION, stream: LogStream::Stdout, offset, max_bytes };
    let growing = LogExtent { generation: GENERATION, len: 100_000, done: false };
    let max = u64::from(LOG_READ_MAX);
    assert_eq!(read(0, LOG_READ_MAX).window(&growing), Ok(0..max));
    let first = LogChunk::new(&growing, 0, vec![b'x'; LOG_READ_MAX as usize]);
    assert_eq!((first.generation, first.offset, first.eof, first.done, first.next()), (GENERATION, 0, false, false, max));
    assert_eq!(read(max, LOG_READ_MAX).window(&growing), Ok(max..100_000));
    let tail = LogChunk::new(&growing, max, vec![b'x'; (100_000 - max) as usize]);
    assert!(tail.eof && !tail.done, "at its end for now, and still growing");
    // At the end a read takes nothing yet; past it a read is refused, never reset.
    assert_eq!(read(100_000, 16).window(&growing), Ok(100_000..100_000));
    assert_eq!(read(100_001, 16).window(&growing).expect_err("past the end").kind(), "artifact::invalid");
    assert_eq!(read(0, 0).window(&growing).expect_err("no bytes").kind(), "artifact::invalid");
    assert_eq!(read(0, LOG_READ_MAX + 1).window(&growing).expect_err("past the maximum").kind(), "artifact::invalid");
    // A finished log's last chunk says so, and an earlier one only that it's final.
    let finished = LogExtent { done: true, ..growing };
    let last = LogChunk::new(&finished, 99_990, vec![b'x'; 10]);
    assert!(last.eof && last.done);
    let early = LogChunk::new(&finished, 0, vec![b'x'; 10]);
    assert!(!early.eof && early.done, "final, with more to read");
}

#[test]
fn a_full_log_chunk_fits_an_rpc_frame_and_the_decode_budget() {
    let extent = LogExtent { generation: GENERATION, len: u64::from(LOG_READ_MAX), done: true };
    let reply = RuntimeReply::Artifact(ArtifactReply::Log(LogChunk::new(&extent, 0, vec![0xff; LOG_READ_MAX as usize])));
    let bytes = bitcode::encode(&reply);
    assert!(bytes.len() < RPC_FRAME_DEFAULT as usize);
    let back: RuntimeReply = decode_payload(&bytes, DECODE_BUDGET_DEFAULT).expect("within the default budget");
    assert_eq!(back, reply);
}

#[test]
fn a_remote_path_is_absolute_posix_and_never_a_client_path() {
    for good in ["/", "/srv/state", "/srv/a\\b", "/srv/C:x", "/srv/report ✓.nuon"] {
        assert_eq!(RemotePath::new(good).expect(good).as_str(), good, "kept as the execution host spells it");
    }
    let long_component = format!("/srv/{}", "c".repeat(COMPONENT_MAX + 1));
    let long_path = format!("/{}", ["c"; PATH_MAX / 2 + 1].join("/"));
    let bad = [
        "",
        "srv/state",
        "C:\\srv",
        "\\\\server\\share",
        "/srv/../etc",
        "/srv/./x",
        "/srv//x",
        "/srv/x/",
        "/srv/\0x",
        &long_component,
        &long_path,
    ];
    for path in bad {
        assert_eq!(RemotePath::new(path).expect_err(path).kind(), "artifact::invalid", "{path:?}");
    }
    // A decoded path is held to the same rules before SFTP opens it.
    let decoded: RemotePath = decode_payload(&bitcode::encode(&"../escape".to_string()), DECODE_BUDGET_DEFAULT)
        .expect("a string's layout");
    assert_eq!(decoded.check().expect_err("relative").kind(), "artifact::invalid");
}

#[test]
fn a_destination_is_held_to_every_client_os_rules() {
    for good in ["out.txt", "sub/dir/out.txt", "report ✓.nuon", "a.b.c", ".hidden", "CONSOLE.txt", "lpt10"] {
        check_dest(good).expect(good);
    }
    let long = "c".repeat(COMPONENT_MAX + 1);
    let bad = [
        "",
        "/abs",
        "../escape",
        "sub/../../escape",
        "a//b",
        "a/",
        "./a",
        "a\\..\\x",
        "C:x",
        "file:stream",
        "CON",
        "con.txt",
        "sub/NUL.tar.gz",
        "LPT1",
        "com9.log",
        "aux .txt",
        "name.",
        "name ",
        "a*b",
        "q?",
        "\"x\"",
        "<x>",
        "a|b",
        "\0x",
        "x\u{1f}",
        &long,
    ];
    for dest in bad {
        assert_eq!(check_dest(dest).expect_err(dest).kind(), "artifact::invalid", "{dest:?}");
    }
}

#[test]
fn an_event_is_refused_an_attachment_no_client_could_keep() {
    let mut backlog = EventBacklog::new(GENERATION, &LIMITS_DEFAULT);
    let attachment = |dest: &str| EventArtifact {
        artifact: ArtifactRef { id: id(1), size: 16, sha256: [0x33; 32] },
        dest: dest.to_string(),
    };
    let payload = || EventPayload::Lease(LeaseNotice::Lost { lease: LEASE, epoch: LeaseEpoch(1), reason: LossReason::Expired });
    let escape = backlog.push(None, payload(), vec![attachment("..\\escape")]).expect_err("an escape on Windows");
    assert_eq!(escape.kind(), "daemon::invalid_frame");
    assert_eq!(backlog.newest(), None, "nothing numbered");
    assert_eq!(backlog.push(None, payload(), vec![attachment("out/report.nuon")]), Ok(1));
}

#[test]
fn a_reference_survives_a_new_lease_and_names_no_client_path() {
    let mut store = store();
    let request = ReserveUpload { display_name: "C:\\Users\\agent\\result.nuon".to_string(), ..upload(16) };
    store.reserve(NOW, LEASE, id(1), &request).expect("reserved");
    let PublishStart::Copy(plan) = store.begin_publish(NOW, LEASE, id(1)).expect("begun") else {
        panic!("a copy to make")
    };
    let published = store.finish_publish(NOW, id(1), copied(&plan)).0.expect("published");
    // The lease taken over, the published object outlives it.
    assert!(store.release(LEASE).is_empty());
    assert_eq!(store.describe(NOW, id(1)), Ok(published.clone()));
    assert_eq!(store.begin_publish(NOW, OTHER, id(1)), Ok(PublishStart::Done(published.clone())));
    // A result or an event names it by id, size, and digest alone, unchanged across the wire.
    let event = EventArtifact { artifact: published.artifact, dest: "result.nuon".to_string() };
    let back: EventArtifact = decode_payload(&bitcode::encode(&event), DECODE_BUDGET_DEFAULT).expect("decoded");
    assert_eq!(back, event);
    let reply = RuntimeReply::Artifact(ArtifactReply::Described(published));
    let back: RuntimeReply = decode_payload(&bitcode::encode(&reply), DECODE_BUDGET_DEFAULT).expect("decoded");
    let RuntimeReply::Artifact(ArtifactReply::Described(described)) = back else { panic!("a description") };
    assert_eq!(described.sftp_path.as_str(), place("objects", 1), "the execution host's path");
    assert_eq!(described.display_name, "C:\\Users\\agent\\result.nuon", "metadata, never a path");
    described.sftp_path.check().expect("held to a remote path's rules on receipt");
}

#[test]
fn recovery_adopts_a_verified_copy_and_quarantines_the_rest() {
    let record = |n: u8, state: ArtifactState| ArtifactRecord {
        artifact: ArtifactRef { id: id(n), size: 16, sha256: [n; 32] },
        purpose: ArtifactPurpose::EquipSource,
        display_name: format!("artifact{n}"),
        state,
        owner: matches!(state, ArtifactState::Reserved | ArtifactState::Publishing).then_some(LEASE),
        deadline_ms: NOW + HOUR,
    };
    let records = [
        // Copied and verified, its metadata commit interrupted.
        record(1, ArtifactState::Publishing),
        // A partial copy.
        record(2, ArtifactState::Publishing),
        // Never copied.
        record(3, ArtifactState::Reserved),
        record(4, ArtifactState::Published),
        // Published, its object since lost.
        record(5, ArtifactState::Published),
        // Collected, its removal interrupted.
        record(6, ArtifactState::Evicted),
    ];
    let hex = |n: u8| id(n).hex();
    let found = Found {
        staging: vec![hex(1), hex(2), hex(3), "stray.part".to_string()],
        objects: vec![hex(1), hex(2), hex(4), hex(6), hex(7), "not-an-id".to_string(), "..".to_string()],
    };
    let mut measured = Vec::new();
    let (store, actions) = ArtifactStore::recover(remote(ROOT), ARTIFACT_LIMITS_DEFAULT, NOW, records, &found, |object| {
        measured.push(object.as_str().to_string());
        if object.as_str() == place("objects", 1) { Some((16, [1; 32])) } else { Some((8, [2; 32])) }
    });
    assert_eq!(measured, [place("objects", 1), place("objects", 2)], "only an unfinished record's object is measured");
    let states: Vec<_> = (1..=7).map(|n| state(&store, NOW, n)).collect();
    use ArtifactState as S;
    assert_eq!(
        states,
        [Some(S::Published), Some(S::Aborted), Some(S::Aborted), Some(S::Published), Some(S::Evicted), Some(S::Evicted), None]
    );
    let at = |path: String| remote(&path);
    assert_eq!(
        actions,
        [
            ArtifactAction::RemoveStaging { path: at(place("staging", 1)) },
            ArtifactAction::RemoveStaging { path: at(place("staging", 2)) },
            ArtifactAction::RemoveStaging { path: at(place("staging", 3)) },
            ArtifactAction::Quarantine { path: at(format!("{ROOT}/staging/stray.part")) },
            ArtifactAction::Quarantine { path: at(place("objects", 2)) },
            ArtifactAction::RemoveObject { path: at(place("objects", 6)) },
            ArtifactAction::Quarantine { path: at(place("objects", 7)) },
            ArtifactAction::Quarantine { path: at(format!("{ROOT}/objects/not-an-id")) },
        ]
    );
    // The adopted object is described as any published one, owned by no lease.
    assert_eq!(store.describe(NOW, id(1)).expect("adopted").artifact.sha256, [1; 32]);
    assert!(store.records().all(|record| record.owner.is_none()));
}

#[test]
fn the_store_bounds_hold_to_the_contract() {
    let d = ARTIFACT_LIMITS_DEFAULT;
    assert_eq!(d.out_of_contract(), None);
    assert_eq!(
        (d.artifact_bytes, d.reservations, d.store_bytes, d.reservation_secs, d.retention_secs),
        (256 << 20, 8, 4 << 30, 3600, 604_800)
    );
    let lower = ArtifactLimits {
        artifact_bytes: 1 << 20,
        reservations: 1,
        store_bytes: 1 << 20,
        reservation_secs: 60,
        retention_secs: 30 * 86_400,
    };
    assert_eq!(lower.out_of_contract(), None, "lower bounds and a longer retention are the worker's to set");
    let cases = [
        ("a zero size", ArtifactLimits { artifact_bytes: 0, ..d }),
        ("a size past its ceiling", ArtifactLimits { artifact_bytes: d.artifact_bytes + 1, ..d }),
        ("no reservations", ArtifactLimits { reservations: 0, ..d }),
        ("reservations past their ceiling", ArtifactLimits { reservations: 9, ..d }),
        ("a store past its ceiling", ArtifactLimits { store_bytes: d.store_bytes + 1, ..d }),
        ("a store short of one artifact", ArtifactLimits { store_bytes: d.artifact_bytes - 1, ..d }),
        ("an expiry past its ceiling", ArtifactLimits { reservation_secs: 3601, ..d }),
        ("a retention short of the window", ArtifactLimits { retention_secs: d.retention_secs - 1, ..d }),
    ];
    for (name, limits) in cases {
        assert!(limits.out_of_contract().is_some(), "{name}");
    }
}

/// Every artifact error, by its variant.
fn errors() -> Vec<(&'static str, ArtifactError)> {
    use ArtifactError as E;
    vec![
        ("invalid", E::Invalid { reason: "a display name of no bytes".to_string() }),
        ("quota", E::Quota { bound: QuotaBound::Store, limit: 1000, requested: 1100 }),
        ("unknown", E::Unknown { id: id(1) }),
        ("foreign", E::Foreign { id: id(1) }),
        ("state", E::State { id: id(1), state: ArtifactState::Evicted }),
        ("mismatch", E::Mismatch { id: id(1), reason: "a short copy".to_string() }),
        ("publish_failed", E::PublishFailed { id: id(1), reason: "fsync failed".to_string() }),
        ("log_unknown", E::LogUnknown { operation: OPERATION }),
    ]
}

/// Every artifact request, by its variant.
fn requests() -> Vec<(&'static str, ArtifactRequest)> {
    let read = ReadLog { operation: OPERATION, stream: LogStream::Stderr, offset: 65_536, max_bytes: LOG_READ_MAX };
    vec![
        ("reserve", ArtifactRequest::Reserve(upload(4096))),
        ("publish", ArtifactRequest::Publish(PublishUpload { id: id(1) })),
        ("inspect", ArtifactRequest::Inspect(InspectArtifact { id: id(1) })),
        ("abort", ArtifactRequest::Abort(AbortUpload { id: id(1) })),
        ("describe", ArtifactRequest::Describe(DescribeArtifact { id: id(1) })),
        ("read_log", ArtifactRequest::ReadLog(read)),
    ]
}

/// Every artifact reply, by its variant, a status's two among them.
fn replies() -> Vec<(&'static str, ArtifactReply)> {
    let description = ArtifactDescription {
        artifact: ArtifactRef { id: id(1), size: 4096, sha256: [0x33; 32] },
        purpose: ArtifactPurpose::EquipSource,
        display_name: "geo.tar".to_string(),
        sftp_path: remote(&place("objects", 1)),
    };
    let reservation = UploadReservation { id: id(1), staging_path: remote(&place("staging", 1)), expires_ms: 3_600_000 };
    let chunk = LogChunk { generation: GENERATION, offset: 65_536, bytes: b"line\n".to_vec(), eof: true, done: false };
    vec![
        ("reserved", ArtifactReply::Reserved(reservation)),
        ("published", ArtifactReply::Published(description.clone())),
        ("inspected/known", ArtifactReply::Inspected(ArtifactStatus::Known { id: id(1), state: ArtifactState::Publishing })),
        ("inspected/unknown", ArtifactReply::Inspected(ArtifactStatus::Unknown { id: id(2) })),
        ("aborted", ArtifactReply::Aborted { id: id(1) }),
        ("described", ArtifactReply::Described(description)),
        ("log", ArtifactReply::Log(chunk)),
    ]
}

#[test]
fn each_error_has_a_stable_kind_and_renders_as_one_row() {
    let named: BTreeSet<_> = errors()
        .iter()
        .map(|(_, e)| match e {
            ArtifactError::Invalid { .. } => "invalid",
            ArtifactError::Quota { .. } => "quota",
            ArtifactError::Unknown { .. } => "unknown",
            ArtifactError::Foreign { .. } => "foreign",
            ArtifactError::State { .. } => "state",
            ArtifactError::Mismatch { .. } => "mismatch",
            ArtifactError::PublishFailed { .. } => "publish_failed",
            ArtifactError::LogUnknown { .. } => "log_unknown",
        })
        .collect();
    assert_eq!(named.len(), 8);
    let kinds: BTreeSet<_> = errors().iter().map(|(_, e)| e.kind()).collect();
    assert_eq!(kinds.len(), 8);
    for (name, error) in errors() {
        assert!(error.kind().starts_with("artifact::"), "{name}");
        let rows = RuntimeError::Artifact(error.clone()).rows();
        assert_eq!(rows.len(), 1, "{name}");
        assert_eq!((rows[0].kind.as_str(), rows[0].message.as_str()), (error.kind(), error.message().as_str()), "{name}");
    }
}

/// Every artifact message's bitcode.
fn vectors() -> BTreeMap<String, golden::Vector> {
    let mut out = BTreeMap::new();
    for (name, request) in requests() {
        out.insert(format!("request/{name}"), golden::vector(&request));
    }
    for (name, reply) in replies() {
        out.insert(format!("reply/{name}"), golden::vector(&reply));
    }
    for (name, error) in errors() {
        out.insert(format!("error/{name}"), golden::vector(&error));
    }
    use ArtifactState as S;
    for state in [S::Reserved, S::Publishing, S::Published, S::Aborted, S::Expired, S::Evicted] {
        out.insert(format!("state/{state:?}"), golden::vector(&state));
    }
    out.insert("limits".to_string(), golden::vector(&ARTIFACT_LIMITS_DEFAULT));
    out
}

#[test]
fn each_message_has_its_golden_vector_and_round_trips() {
    let actual = golden::hex_table(vectors());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an artifact message's bytes changed; a deliberate change re-blesses {GOLDEN}");
    let covered: BTreeSet<_> = requests()
        .iter()
        .map(|(_, r)| match r {
            ArtifactRequest::Reserve(_) => "reserve",
            ArtifactRequest::Publish(_) => "publish",
            ArtifactRequest::Inspect(_) => "inspect",
            ArtifactRequest::Abort(_) => "abort",
            ArtifactRequest::Describe(_) => "describe",
            ArtifactRequest::ReadLog(_) => "read_log",
        })
        .collect();
    assert_eq!(covered.len(), 6);
    let covered: BTreeSet<_> = replies()
        .iter()
        .map(|(_, r)| match r {
            ArtifactReply::Reserved(_) => "reserved",
            ArtifactReply::Published(_) => "published",
            ArtifactReply::Inspected(_) => "inspected",
            ArtifactReply::Aborted { .. } => "aborted",
            ArtifactReply::Described(_) => "described",
            ArtifactReply::Log(_) => "log",
        })
        .collect();
    assert_eq!(covered.len(), 6);
    for (name, request) in requests() {
        let back: ArtifactRequest = decode_payload(&bitcode::encode(&request), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, request, "{name}");
    }
    for (name, reply) in replies() {
        let back: ArtifactReply = decode_payload(&bitcode::encode(&reply), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, reply, "{name}");
    }
    for (name, error) in errors() {
        let back: ArtifactError = decode_payload(&bitcode::encode(&error), DECODE_BUDGET_DEFAULT).expect(name);
        assert_eq!(back, error, "{name}");
    }
}
