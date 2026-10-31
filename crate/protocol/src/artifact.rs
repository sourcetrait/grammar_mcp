//! Artifacts: metadata RPCs for what SFTP carries, log reads, and the store.
use crate::*;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

/// An artifact's largest size by default.
pub const ARTIFACT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// The most reservations a worker holds unfinished at once by default.
pub const ACTIVE_RESERVATIONS: u32 = 8;

/// The store's quota by default.
pub const STORE_QUOTA_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// How long an unfinished reservation lives by default.
pub const RESERVATION_EXPIRY_SECS: u32 = 3600;

/// How long a published artifact is kept at least: a result's window.
pub const RESULT_RETENTION_SECS: u32 = 7 * 24 * 3600;

/// The largest SFTP packet either side takes.
pub const SFTP_PACKET_MAX: u32 = 1024 * 1024;

/// The largest SFTP data chunk either side takes.
pub const SFTP_CHUNK_MAX: u32 = 64 * 1024;

/// The most bytes one log read returns.
pub const LOG_READ_MAX: u32 = 64 * 1024;

/// A display name's longest, in bytes.
pub const DISPLAY_NAME_MAX: usize = 255;

/// A remote path's or a destination's longest, in bytes.
pub const PATH_MAX: usize = 4096;

/// A component's longest, in bytes, in a remote path or a destination.
pub const COMPONENT_MAX: usize = 255;

/// The SHA-256 of no bytes, the digest an empty artifact declares.
pub const EMPTY_SHA256: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];

/// The Windows device names no destination's component may be, in any case.
const DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The characters a component may not hold on some client OS.
const UNPORTABLE: [char; 8] = ['\\', ':', '*', '?', '"', '<', '>', '|'];

/// The bounds a worker's artifact store keeps, advertised in its hello.
#[cereal::derived(Data, Copy, Eq)]
pub struct ArtifactLimits {
    /// One artifact's largest size.
    pub artifact_bytes: u64,
    /// The reservations held unfinished at once.
    pub reservations: u32,
    /// The bytes every unfinished reservation and publication holds together.
    pub store_bytes: u64,
    /// How long a reservation waits for its publication.
    pub reservation_secs: u32,
    /// How long a published artifact is kept at least.
    pub retention_secs: u32,
}

/// The store's bounds by default, each its ceiling, the retention its floor.
pub const ARTIFACT_LIMITS_DEFAULT: ArtifactLimits = ArtifactLimits {
    artifact_bytes: ARTIFACT_MAX_BYTES,
    reservations: ACTIVE_RESERVATIONS,
    store_bytes: STORE_QUOTA_BYTES,
    reservation_secs: RESERVATION_EXPIRY_SECS,
    retention_secs: RESULT_RETENTION_SECS,
};

impl ArtifactLimits {
    /// The first bound out of contract, said, or none.
    pub fn out_of_contract(&self) -> Option<String> {
        let ceiling = ARTIFACT_LIMITS_DEFAULT;
        let bounds = [
            ("artifact_bytes", self.artifact_bytes, ceiling.artifact_bytes),
            ("reservations", u64::from(self.reservations), u64::from(ceiling.reservations)),
            ("store_bytes", self.store_bytes, ceiling.store_bytes),
            ("reservation_secs", u64::from(self.reservation_secs), u64::from(ceiling.reservation_secs)),
        ];
        for (name, value, ceiling) in bounds {
            if value == 0 {
                return Some(format!("an artifact {name} of zero"));
            }
            if value > ceiling {
                return Some(format!("an artifact {name} of {value} past its ceiling {ceiling}"));
            }
        }
        if self.store_bytes < self.artifact_bytes {
            return Some(format!(
                "an artifact store of {} bytes, short of one {}-byte artifact",
                self.store_bytes, self.artifact_bytes
            ));
        }
        if self.retention_secs < ceiling.retention_secs {
            return Some(format!(
                "an artifact retention of {}s, short of the {}s result window",
                self.retention_secs, ceiling.retention_secs
            ));
        }
        None
    }
}

/// A POSIX path on the execution host, never one on the frontend's machine.
#[cereal::derived(Data, Eq, Hash)]
pub struct RemotePath(String);

impl RemotePath {
    /// `path` held to a remote path's rules.
    pub fn new(path: &str) -> Result<Self, ArtifactError> {
        let path = Self(path.to_string());
        path.check().map(|()| path)
    }

    /// Absolute, plain components, no NUL, within PATH_MAX: a decoded path's check.
    pub fn check(&self) -> Result<(), ArtifactError> {
        let invalid = |reason: &str| Err(ArtifactError::Invalid { reason: format!("a remote path {reason}") });
        if self.0.len() > PATH_MAX {
            return invalid("past PATH_MAX");
        }
        let Some(rest) = self.0.strip_prefix('/') else {
            return invalid("that is not absolute");
        };
        if self.0.contains('\0') {
            return invalid("holding a NUL");
        }
        if rest.is_empty() {
            return Ok(());
        }
        for component in rest.split('/') {
            match component {
                "" => return invalid("holding an empty component"),
                "." | ".." => return invalid("holding a relative component"),
                long if long.len() > COMPONENT_MAX => return invalid("holding a component past 255 bytes"),
                _ => {}
            }
        }
        Ok(())
    }

    /// The path as the execution host reads it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `name`, a plain component the store chose, beneath this path.
    fn child(&self, name: &str) -> Self {
        if self.0 == "/" { Self(format!("/{name}")) } else { Self(format!("{}/{name}", self.0)) }
    }
}

impl fmt::Display for RemotePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An artifact's opaque identity, scoped to its worker's instance.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct ArtifactId(pub [u8; 16]);

impl ArtifactId {
    /// The artifact's lowercase hex.
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The id a store's file name spells in 32 lowercase hex digits.
    pub fn from_hex(name: &str) -> Option<Self> {
        let digits = name.as_bytes();
        if digits.len() != 32 {
            return None;
        }
        let mut id = [0u8; 16];
        for (byte, [high, low]) in id.iter_mut().zip(digits.as_chunks::<2>().0) {
            *byte = (nibble(*high)? << 4) | nibble(*low)?;
        }
        Some(Self(id))
    }
}

/// One lowercase hex digit's value.
fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

/// What an artifact is for.
#[cereal::derived(Data, Copy, Eq)]
pub enum ArtifactPurpose {
    /// An equip's source, uploaded before the runtime installs from it.
    EquipSource,
    /// A file a remote Channel delivered, exposed for the frontend to fetch.
    ChannelAttachment,
    /// A result too large for a reply.
    OperationResult,
    /// A finished operation's log, published whole.
    OperationLog,
}

impl ArtifactPurpose {
    /// Whether a client uploads it; the worker publishes every other purpose.
    pub fn uploadable(self) -> bool {
        matches!(self, Self::EquipSource)
    }
}

/// An artifact's place in its lifecycle.
#[cereal::derived(Data, Copy, Eq)]
pub enum ArtifactState {
    /// Reserved, its upload unpublished.
    Reserved,
    /// Its staged upload being copied and verified.
    Publishing,
    /// Immutable and readable over SFTP.
    Published,
    /// Its reservation abandoned before publication.
    Aborted,
    /// Its reservation lapsed before publication.
    Expired,
    /// Published once, and since collected.
    Evicted,
}

/// What a result or an attachment names an artifact by: identity, size, hash.
#[cereal::derived(Data, Copy, Eq)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// A display name held to its rules: 1 to 255 bytes, no control character.
pub fn check_display_name(name: &str) -> Result<(), ArtifactError> {
    let invalid = |reason: String| Err(ArtifactError::Invalid { reason });
    if name.is_empty() {
        return invalid("a display name of no bytes".to_string());
    }
    if name.len() > DISPLAY_NAME_MAX {
        return invalid(format!("a display name of {} bytes past {DISPLAY_NAME_MAX}", name.len()));
    }
    if name.chars().any(char::is_control) {
        return invalid("a display name holding a control character".to_string());
    }
    Ok(())
}

/// A declared digest held to its size: never zeros, the empty one for no bytes.
fn check_digest(size: u64, sha256: &[u8; 32]) -> Result<(), ArtifactError> {
    if *sha256 == [0; 32] {
        return Err(ArtifactError::Invalid { reason: "a placeholder digest of zeros".to_string() });
    }
    if size == 0 && *sha256 != EMPTY_SHA256 {
        return Err(ArtifactError::Invalid { reason: "an empty artifact whose digest isn't the empty one".to_string() });
    }
    Ok(())
}

/// A destination held to the rules every client OS keeps beneath its inbox.
pub fn check_dest(dest: &str) -> Result<(), ArtifactError> {
    let invalid = |reason: &str| Err(ArtifactError::Invalid { reason: format!("a destination {reason}") });
    if dest.is_empty() {
        return invalid("of no bytes");
    }
    if dest.len() > PATH_MAX {
        return invalid("past PATH_MAX");
    }
    for component in dest.split('/') {
        if component.is_empty() {
            return invalid("that is absolute or holds an empty component");
        }
        if component == "." || component == ".." {
            return invalid("holding a relative component");
        }
        if component.len() > COMPONENT_MAX {
            return invalid("holding a component past 255 bytes");
        }
        if component.chars().any(|c| c.is_control() || UNPORTABLE.contains(&c)) {
            return invalid("holding a character some client OS refuses");
        }
        if component.ends_with('.') || component.ends_with(' ') {
            return invalid("holding a component Windows would rename");
        }
        let stem = component.split('.').next().unwrap_or(component).trim_end_matches(' ');
        if DEVICE_NAMES.iter().any(|device| stem.eq_ignore_ascii_case(device)) {
            return invalid("naming a Windows device");
        }
    }
    Ok(())
}

/// A reservation for an upload: its purpose, name, size, and hash, declared first.
#[cereal::derived(Data, Eq)]
pub struct ReserveUpload {
    pub purpose: ArtifactPurpose,
    /// Metadata alone, never a path component.
    pub display_name: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

impl ReserveUpload {
    /// The reservation's metadata held to the contract and to `limits`.
    pub fn check(&self, limits: &ArtifactLimits) -> Result<(), ArtifactError> {
        if !self.purpose.uploadable() {
            return Err(ArtifactError::Invalid {
                reason: format!("a {:?} artifact, which the worker alone publishes", self.purpose),
            });
        }
        check_display_name(&self.display_name)?;
        check_digest(self.size, &self.sha256)?;
        if self.size > limits.artifact_bytes {
            return Err(ArtifactError::Quota {
                bound: QuotaBound::Artifact,
                limit: limits.artifact_bytes,
                requested: self.size,
            });
        }
        Ok(())
    }
}

/// A reservation granted: the identity, the staging path SFTP writes, its expiry.
#[cereal::derived(Data, Eq)]
pub struct UploadReservation {
    pub id: ArtifactId,
    pub staging_path: RemotePath,
    /// How long the reservation waits for its publication.
    pub expires_ms: u32,
}

/// A staged upload's publication asked for, once SFTP acknowledged every write.
#[cereal::derived(Data, Copy, Eq)]
pub struct PublishUpload {
    pub id: ArtifactId,
}

/// An artifact's state asked for: how an uncertain publication is resolved.
#[cereal::derived(Data, Copy, Eq)]
pub struct InspectArtifact {
    pub id: ArtifactId,
}

/// An unfinished reservation's abandonment asked for.
#[cereal::derived(Data, Copy, Eq)]
pub struct AbortUpload {
    pub id: ArtifactId,
}

/// A published artifact's description asked for.
#[cereal::derived(Data, Copy, Eq)]
pub struct DescribeArtifact {
    pub id: ArtifactId,
}

/// A published artifact as its worker describes it.
#[cereal::derived(Data, Eq)]
pub struct ArtifactDescription {
    pub artifact: ArtifactRef,
    pub purpose: ArtifactPurpose,
    pub display_name: String,
    /// The execution host's path SFTP reads it from.
    pub sftp_path: RemotePath,
}

/// What a lookup knows of an artifact, which moves nothing.
#[cereal::derived(Data, Copy, Eq)]
pub enum ArtifactStatus {
    Known { id: ArtifactId, state: ArtifactState },
    Unknown { id: ArtifactId },
}

/// Which of an operation's streams a log read takes.
#[cereal::derived(Data, Copy, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
}

/// A bounded read of a running operation's log.
#[cereal::derived(Data, Copy, Eq)]
pub struct ReadLog {
    pub operation: OperationId,
    pub stream: LogStream,
    pub offset: u64,
    /// At most `LOG_READ_MAX`.
    pub max_bytes: u32,
}

/// A log as its reader finds it: its writer, its length, and whether it's final.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogExtent {
    pub generation: WorkerGeneration,
    pub len: u64,
    /// The operation ended before `len` was taken, so `len` is the whole log.
    pub done: bool,
}

impl ReadLog {
    /// The bytes this read takes of a log at `extent`, or refused.
    pub fn window(&self, extent: &LogExtent) -> Result<Range<u64>, ArtifactError> {
        let invalid = |reason: String| Err(ArtifactError::Invalid { reason });
        if self.max_bytes == 0 {
            return invalid("a log read of no bytes".to_string());
        }
        if self.max_bytes > LOG_READ_MAX {
            return invalid(format!("a log read of {} bytes past {LOG_READ_MAX}", self.max_bytes));
        }
        if self.offset > extent.len {
            return invalid(format!("a log read at {} past the log's {} bytes", self.offset, extent.len));
        }
        Ok(self.offset..extent.len.min(self.offset.saturating_add(u64::from(self.max_bytes))))
    }
}

/// One log read's bytes from its offset, and whether more may come.
#[cereal::derived(Data, Eq)]
pub struct LogChunk {
    /// The worker generation whose operation wrote the log.
    pub generation: WorkerGeneration,
    /// Where the bytes start.
    pub offset: u64,
    pub bytes: Vec<u8>,
    /// The log holds nothing past these bytes yet.
    pub eof: bool,
    /// The operation ended; the log will grow no further.
    pub done: bool,
}

impl LogChunk {
    /// The chunk `bytes`, the runtime's read from `offset`, make of `extent`.
    pub fn new(extent: &LogExtent, offset: u64, bytes: Vec<u8>) -> Self {
        let end = offset.saturating_add(bytes.len() as u64);
        Self { generation: extent.generation, offset, eof: end >= extent.len, done: extent.done, bytes }
    }

    /// Where the next read starts.
    pub fn next(&self) -> u64 {
        self.offset.saturating_add(self.bytes.len() as u64)
    }
}

/// An artifact request.
#[cereal::derived(Data, Eq)]
pub enum ArtifactRequest {
    Reserve(ReserveUpload),
    Publish(PublishUpload),
    Inspect(InspectArtifact),
    Abort(AbortUpload),
    Describe(DescribeArtifact),
    ReadLog(ReadLog),
}

/// An artifact request's answer.
#[cereal::derived(Data, Eq)]
pub enum ArtifactReply {
    Reserved(UploadReservation),
    Published(ArtifactDescription),
    Inspected(ArtifactStatus),
    Aborted { id: ArtifactId },
    Described(ArtifactDescription),
    Log(LogChunk),
}

/// Which of the store's bounds a request would pass.
#[cereal::derived(Data, Copy, Eq)]
pub enum QuotaBound {
    /// One artifact's size.
    Artifact,
    /// The unfinished reservations.
    Reservations,
    /// The bytes the store holds.
    Store,
}

impl QuotaBound {
    /// The bound's name, as its limit's field names it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Artifact => "artifact_bytes",
            Self::Reservations => "reservations",
            Self::Store => "store_bytes",
        }
    }
}

/// Why an artifact request was refused, nothing changed by it.
#[cereal::derived(Data, Eq)]
pub enum ArtifactError {
    /// Metadata a rule refuses: a purpose, name, digest, path, or log read.
    Invalid { reason: String },
    /// A bound of the store's that the request would pass.
    Quota { bound: QuotaBound, limit: u64, requested: u64 },
    /// No artifact of this worker's has the id.
    Unknown { id: ArtifactId },
    /// The reservation is another lease's.
    Foreign { id: ArtifactId },
    /// The artifact's state doesn't take the request.
    State { id: ArtifactId, state: ArtifactState },
    /// The staged upload is not what its reservation declared.
    Mismatch { id: ArtifactId, reason: String },
    /// The publication's copy failed before anything was published.
    PublishFailed { id: ArtifactId, reason: String },
    /// No log of this worker's is the operation's.
    LogUnknown { operation: OperationId },
}

impl ArtifactError {
    /// The stable kind the frontend's envelope reports.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Invalid { .. } => "artifact::invalid",
            Self::Quota { .. } => "artifact::quota",
            Self::Unknown { .. } => "artifact::unknown",
            Self::Foreign { .. } => "artifact::foreign",
            Self::State { .. } => "artifact::state",
            Self::Mismatch { .. } => "artifact::mismatch",
            Self::PublishFailed { .. } => "artifact::publish_failed",
            Self::LogUnknown { .. } => "artifact::log_unknown",
        }
    }

    /// The message the frontend's envelope reports.
    pub fn message(&self) -> String {
        match self {
            Self::Invalid { reason } => format!("an invalid artifact request: {reason}"),
            Self::Quota { bound, limit, requested } => {
                format!("the request needs {requested} where the store's {} is {limit}", bound.name())
            }
            Self::Unknown { id } => format!("no artifact {} on this worker", id.hex()),
            Self::Foreign { id } => format!("artifact {} is another lease's reservation", id.hex()),
            Self::State { id, state } => format!("artifact {} is {state:?}, which the request can't take", id.hex()),
            Self::Mismatch { id, reason } => format!("artifact {} wasn't published: {reason}", id.hex()),
            Self::PublishFailed { id, reason } => {
                format!("artifact {}'s publication failed, nothing published: {reason}", id.hex())
            }
            Self::LogUnknown { operation } => format!("no log of operation {} on this worker", operation.hex()),
        }
    }
}

/// What keeps a published artifact from collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactHold {
    /// A retained operation result names it.
    Result(OperationId),
    /// A pending event's attachment manifest names it, by the event's sequence.
    Event(u64),
    /// A reader has it open, a transfer to the frontend among them.
    Reader(u64),
}

/// What the runtime does as the store changes, once the change is journaled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactAction {
    /// Remove a reservation's staging file: aborted, lapsed, or published.
    RemoveStaging { path: RemotePath },
    /// Remove a collected artifact's object.
    RemoveObject { path: RemotePath },
    /// Set aside a file the store can't account for, never promoted.
    Quarantine { path: RemotePath },
}

/// A publication for the runtime to carry out: what it copies, and into where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPlan {
    pub id: ArtifactId,
    pub staging: RemotePath,
    pub object: RemotePath,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// What a publish request comes to before any copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishStart {
    /// The copy to make.
    Copy(PublishPlan),
    /// Published before: the same object again.
    Done(ArtifactDescription),
}

/// The runtime's copy of a staged upload, as it measured it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    /// Copied into a new inode and synced: the bytes it read and their digest.
    Copied { size: u64, sha256: [u8; 32] },
    /// The copy failed: a link, an unexpected inode, a full disk, a failed sync.
    Failed { reason: String },
}

/// One artifact as the store keeps it and its journal records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact: ArtifactRef,
    pub purpose: ArtifactPurpose,
    pub display_name: String,
    pub state: ArtifactState,
    /// The lease whose reservation it is, while that lease holds it.
    pub owner: Option<LeaseId>,
    /// A reservation's lapse, or the end of a publication's retention.
    pub deadline_ms: u64,
}

/// The file names a worker's start finds beneath its store's root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Found {
    pub staging: Vec<String>,
    pub objects: Vec<String>,
}

/// An artifact's record and what holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    record: ArtifactRecord,
    holds: HashSet<ArtifactHold>,
}

impl Entry {
    /// The entry's state at `now_ms`: a reservation past its deadline lapsed.
    fn state(&self, now_ms: u64) -> ArtifactState {
        match self.record.state {
            ArtifactState::Reserved if now_ms >= self.record.deadline_ms => ArtifactState::Expired,
            state => state,
        }
    }

    /// The entry described, its object beneath `root`.
    fn description(&self, root: &RemotePath) -> ArtifactDescription {
        ArtifactDescription {
            artifact: self.record.artifact,
            purpose: self.record.purpose,
            display_name: self.record.display_name.clone(),
            sftp_path: object_in(root, self.record.artifact.id),
        }
    }
}

/// Seconds as milliseconds.
fn millis(secs: u32) -> u64 {
    u64::from(secs) * 1000
}

/// `id`'s staging file beneath `root`.
fn staging_in(root: &RemotePath, id: ArtifactId) -> RemotePath {
    root.child("staging").child(&id.hex())
}

/// `id`'s object beneath `root`.
fn object_in(root: &RemotePath, id: ArtifactId) -> RemotePath {
    root.child("objects").child(&id.hex())
}

/// Whether a listed name is one plain component.
fn plain(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\0'])
}

/// A worker's artifacts: each reservation, publication, and tombstone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStore {
    root: RemotePath,
    limits: ArtifactLimits,
    entries: HashMap<ArtifactId, Entry>,
    /// A recovered store's holds not yet taken again, collection evicting nothing.
    restoring: bool,
}

impl ArtifactStore {
    /// An empty store beneath `root`, the worker's durable state directory.
    pub fn new(root: RemotePath, limits: ArtifactLimits) -> Self {
        Self { root, limits, entries: HashMap::new(), restoring: false }
    }

    /// A recovered store's durable holds taken again: collection may evict from now.
    pub fn holds_restored(&mut self) {
        self.restoring = false;
    }

    /// The bounds the store keeps.
    pub fn limits(&self) -> ArtifactLimits {
        self.limits
    }

    /// The staging file SFTP writes for `id`'s reservation.
    pub fn staging_path(&self, id: ArtifactId) -> RemotePath {
        staging_in(&self.root, id)
    }

    /// The object SFTP reads once `id` is published.
    pub fn object_path(&self, id: ArtifactId) -> RemotePath {
        object_in(&self.root, id)
    }

    /// Every record, for the journal.
    pub fn records(&self) -> impl Iterator<Item = &ArtifactRecord> {
        self.entries.values().map(|entry| &entry.record)
    }

    /// The ids held, in order.
    fn ids(&self) -> Vec<ArtifactId> {
        let mut ids: Vec<_> = self.entries.keys().copied().collect();
        ids.sort_by_key(|id| id.0);
        ids
    }

    /// The unfinished reservations at `now_ms`, and the bytes the store holds.
    fn usage(&self, now_ms: u64) -> (u32, u64) {
        self.entries.values().fold((0, 0), |(count, bytes), entry| match entry.state(now_ms) {
            ArtifactState::Reserved | ArtifactState::Publishing => (count + 1, bytes + entry.record.artifact.size),
            ArtifactState::Published => (count, bytes + entry.record.artifact.size),
            _ => (count, bytes),
        })
    }

    /// The store's bytes refused past its quota with `size` more.
    fn fits(&self, now_ms: u64, size: u64) -> Result<(), ArtifactError> {
        let requested = self.usage(now_ms).1.saturating_add(size);
        if requested > self.limits.store_bytes {
            return Err(ArtifactError::Quota { bound: QuotaBound::Store, limit: self.limits.store_bytes, requested });
        }
        Ok(())
    }

    /// `request` reserved under `lease` as `id`, fresh from the OS's CSPRNG.
    pub fn reserve(
        &mut self,
        now_ms: u64,
        lease: LeaseId,
        id: ArtifactId,
        request: &ReserveUpload,
    ) -> Result<UploadReservation, ArtifactError> {
        request.check(&self.limits)?;
        if self.entries.contains_key(&id) {
            return Err(ArtifactError::Invalid { reason: format!("an id {} already held", id.hex()) });
        }
        let count = self.usage(now_ms).0;
        if count >= self.limits.reservations {
            return Err(ArtifactError::Quota {
                bound: QuotaBound::Reservations,
                limit: u64::from(self.limits.reservations),
                requested: u64::from(count) + 1,
            });
        }
        self.fits(now_ms, request.size)?;
        let expires = millis(self.limits.reservation_secs);
        let record = ArtifactRecord {
            artifact: ArtifactRef { id, size: request.size, sha256: request.sha256 },
            purpose: request.purpose,
            display_name: request.display_name.clone(),
            state: ArtifactState::Reserved,
            owner: Some(lease),
            deadline_ms: now_ms.saturating_add(expires),
        };
        self.entries.insert(id, Entry { record, holds: HashSet::new() });
        Ok(UploadReservation {
            id,
            staging_path: self.staging_path(id),
            expires_ms: u32::try_from(expires).unwrap_or(u32::MAX),
        })
    }

    /// A publish of `id` under `lease`: the copy, the same object, or refused.
    pub fn begin_publish(&mut self, now_ms: u64, lease: LeaseId, id: ArtifactId) -> Result<PublishStart, ArtifactError> {
        let entry = self.entries.get_mut(&id).ok_or(ArtifactError::Unknown { id })?;
        match entry.state(now_ms) {
            ArtifactState::Published => return Ok(PublishStart::Done(entry.description(&self.root))),
            ArtifactState::Reserved if entry.record.owner != Some(lease) => return Err(ArtifactError::Foreign { id }),
            ArtifactState::Reserved => {}
            state => return Err(ArtifactError::State { id, state }),
        }
        entry.record.state = ArtifactState::Publishing;
        Ok(PublishStart::Copy(PublishPlan {
            id,
            staging: staging_in(&self.root, id),
            object: object_in(&self.root, id),
            size: entry.record.artifact.size,
            sha256: entry.record.artifact.sha256,
        }))
    }

    /// `id`'s publication ended as the runtime measured its copy.
    pub fn finish_publish(
        &mut self,
        now_ms: u64,
        id: ArtifactId,
        outcome: PublishOutcome,
    ) -> (Result<ArtifactDescription, ArtifactError>, Vec<ArtifactAction>) {
        let retention = millis(self.limits.retention_secs);
        let Some(entry) = self.entries.get_mut(&id) else {
            return (Err(ArtifactError::Unknown { id }), Vec::new());
        };
        if entry.record.state != ArtifactState::Publishing {
            return (Err(ArtifactError::State { id, state: entry.state(now_ms) }), Vec::new());
        }
        let declared = entry.record.artifact;
        let refusal = match outcome {
            PublishOutcome::Copied { size, sha256 } if size == declared.size && sha256 == declared.sha256 => None,
            PublishOutcome::Copied { size, .. } if size != declared.size => Some(ArtifactError::Mismatch {
                id,
                reason: format!("the staged file holds {size} bytes where {} were declared", declared.size),
            }),
            PublishOutcome::Copied { .. } => Some(ArtifactError::Mismatch {
                id,
                reason: "the staged file's digest isn't the declared one".to_string(),
            }),
            PublishOutcome::Failed { reason } => Some(ArtifactError::PublishFailed { id, reason }),
        };
        let staging = staging_in(&self.root, id);
        match refusal {
            None => {
                entry.record.state = ArtifactState::Published;
                entry.record.owner = None;
                entry.record.deadline_ms = now_ms.saturating_add(retention);
                (Ok(entry.description(&self.root)), vec![ArtifactAction::RemoveStaging { path: staging }])
            }
            Some(refusal) if entry.record.owner.is_some() => {
                entry.record.state = ArtifactState::Reserved;
                (Err(refusal), Vec::new())
            }
            Some(refusal) => {
                entry.record.state = ArtifactState::Aborted;
                (Err(refusal), vec![ArtifactAction::RemoveStaging { path: staging }])
            }
        }
    }

    /// `id`'s unfinished reservation abandoned by `lease`, its quota freed.
    pub fn abort(
        &mut self,
        now_ms: u64,
        lease: LeaseId,
        id: ArtifactId,
    ) -> (Result<ArtifactId, ArtifactError>, Vec<ArtifactAction>) {
        let staging = staging_in(&self.root, id);
        let Some(entry) = self.entries.get_mut(&id) else {
            return (Err(ArtifactError::Unknown { id }), Vec::new());
        };
        match entry.state(now_ms) {
            ArtifactState::Aborted => (Ok(id), Vec::new()),
            ArtifactState::Reserved if entry.record.owner != Some(lease) => {
                (Err(ArtifactError::Foreign { id }), Vec::new())
            }
            ArtifactState::Reserved => {
                entry.record.state = ArtifactState::Aborted;
                entry.record.owner = None;
                (Ok(id), vec![ArtifactAction::RemoveStaging { path: staging }])
            }
            state => (Err(ArtifactError::State { id, state }), Vec::new()),
        }
    }

    /// A lost lease's reservations abandoned, its publications left unowned.
    pub fn release(&mut self, lease: LeaseId) -> Vec<ArtifactAction> {
        let mut actions = Vec::new();
        for id in self.ids() {
            let entry = self.entries.get_mut(&id).expect("listed");
            if entry.record.owner != Some(lease) {
                continue;
            }
            entry.record.owner = None;
            if entry.record.state == ArtifactState::Reserved {
                entry.record.state = ArtifactState::Aborted;
                actions.push(ArtifactAction::RemoveStaging { path: staging_in(&self.root, id) });
            }
        }
        actions
    }

    /// What is known of `id` at `now_ms`, which moves nothing.
    pub fn inspect(&self, now_ms: u64, id: ArtifactId) -> ArtifactStatus {
        match self.entries.get(&id) {
            Some(entry) => ArtifactStatus::Known { id, state: entry.state(now_ms) },
            None => ArtifactStatus::Unknown { id },
        }
    }

    /// `id` described while it is published.
    pub fn describe(&self, now_ms: u64, id: ArtifactId) -> Result<ArtifactDescription, ArtifactError> {
        let entry = self.entries.get(&id).ok_or(ArtifactError::Unknown { id })?;
        match entry.state(now_ms) {
            ArtifactState::Published => Ok(entry.description(&self.root)),
            state => Err(ArtifactError::State { id, state }),
        }
    }

    /// An object the worker wrote itself at `object_path`, published.
    pub fn publish_local(
        &mut self,
        now_ms: u64,
        artifact: ArtifactRef,
        purpose: ArtifactPurpose,
        display_name: &str,
    ) -> Result<ArtifactDescription, ArtifactError> {
        let id = artifact.id;
        if purpose.uploadable() {
            return Err(ArtifactError::Invalid { reason: format!("a {purpose:?} artifact, which comes by upload") });
        }
        check_display_name(display_name)?;
        check_digest(artifact.size, &artifact.sha256)?;
        if self.entries.contains_key(&id) {
            return Err(ArtifactError::Invalid { reason: format!("an id {} already held", id.hex()) });
        }
        if artifact.size > self.limits.artifact_bytes {
            return Err(ArtifactError::Quota {
                bound: QuotaBound::Artifact,
                limit: self.limits.artifact_bytes,
                requested: artifact.size,
            });
        }
        self.fits(now_ms, artifact.size)?;
        let record = ArtifactRecord {
            artifact,
            purpose,
            display_name: display_name.to_string(),
            state: ArtifactState::Published,
            owner: None,
            deadline_ms: now_ms.saturating_add(millis(self.limits.retention_secs)),
        };
        let entry = Entry { record, holds: HashSet::new() };
        let description = entry.description(&self.root);
        self.entries.insert(id, entry);
        Ok(description)
    }

    /// `hold` taken on published `id`, which collection then leaves.
    pub fn hold(&mut self, id: ArtifactId, hold: ArtifactHold) -> Result<(), ArtifactError> {
        let entry = self.entries.get_mut(&id).ok_or(ArtifactError::Unknown { id })?;
        if entry.record.state != ArtifactState::Published {
            return Err(ArtifactError::State { id, state: entry.record.state });
        }
        entry.holds.insert(hold);
        Ok(())
    }

    /// `hold` on `id` let go: whether it was held.
    pub fn unhold(&mut self, id: ArtifactId, hold: ArtifactHold) -> bool {
        self.entries.get_mut(&id).is_some_and(|entry| entry.holds.remove(&hold))
    }

    /// Lapsed reservations expired and unheld artifacts past retention collected.
    pub fn collect(&mut self, now_ms: u64) -> Vec<ArtifactAction> {
        let mut actions = Vec::new();
        for id in self.ids() {
            let entry = self.entries.get_mut(&id).expect("listed");
            match entry.record.state {
                ArtifactState::Reserved if now_ms >= entry.record.deadline_ms => {
                    entry.record.state = ArtifactState::Expired;
                    entry.record.owner = None;
                    actions.push(ArtifactAction::RemoveStaging { path: staging_in(&self.root, id) });
                }
                ArtifactState::Published
                    if !self.restoring && now_ms >= entry.record.deadline_ms && entry.holds.is_empty() =>
                {
                    entry.record.state = ArtifactState::Evicted;
                    actions.push(ArtifactAction::RemoveObject { path: object_in(&self.root, id) });
                }
                _ => {}
            }
        }
        actions
    }

    /// A store rebuilt at a worker's start from its journal and its files.
    pub fn recover(
        root: RemotePath,
        limits: ArtifactLimits,
        now_ms: u64,
        records: impl IntoIterator<Item = ArtifactRecord>,
        found: &Found,
        mut measure: impl FnMut(&RemotePath) -> Option<(u64, [u8; 32])>,
    ) -> (Self, Vec<ArtifactAction>) {
        let mut store = Self::new(root, limits);
        store.restoring = true;
        for record in records {
            store.entries.insert(record.artifact.id, Entry { record, holds: HashSet::new() });
        }
        let objects: HashSet<&str> = found.objects.iter().map(String::as_str).collect();
        let retention = millis(limits.retention_secs);
        for id in store.ids() {
            let entry = store.entries.get_mut(&id).expect("listed");
            let present = objects.contains(id.hex().as_str());
            match entry.record.state {
                ArtifactState::Reserved | ArtifactState::Publishing => {
                    let declared = entry.record.artifact;
                    let verified = present && measure(&object_in(&store.root, id)) == Some((declared.size, declared.sha256));
                    entry.record.owner = None;
                    if verified {
                        entry.record.state = ArtifactState::Published;
                        entry.record.deadline_ms = now_ms.saturating_add(retention);
                    } else {
                        entry.record.state = ArtifactState::Aborted;
                    }
                }
                ArtifactState::Published if !present => entry.record.state = ArtifactState::Evicted,
                _ => {}
            }
        }
        let mut actions = Vec::new();
        for name in found.staging.iter().filter(|name| plain(name)) {
            let path = store.root.child("staging").child(name);
            actions.push(match ArtifactId::from_hex(name).filter(|id| store.entries.contains_key(id)) {
                Some(_) => ArtifactAction::RemoveStaging { path },
                None => ArtifactAction::Quarantine { path },
            });
        }
        for name in found.objects.iter().filter(|name| plain(name)) {
            let path = store.root.child("objects").child(name);
            let state = ArtifactId::from_hex(name).and_then(|id| store.entries.get(&id)).map(|entry| entry.record.state);
            match state {
                Some(ArtifactState::Published) => {}
                Some(ArtifactState::Evicted) => actions.push(ArtifactAction::RemoveObject { path }),
                _ => actions.push(ArtifactAction::Quarantine { path }),
            }
        }
        (store, actions)
    }
}
