use crate::*;

/// An artifact's largest size by default.
pub const ARTIFACT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// The most reservations a worker holds unfinished at once by default.
pub const ACTIVE_RESERVATIONS: u32 = 8;

/// The store's quota by default.
pub const STORE_QUOTA_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// How long an unfinished reservation lives by default.
pub const RESERVATION_EXPIRY_SECS: u64 = 3600;

/// The largest SFTP packet either side takes.
pub const SFTP_PACKET_MAX: u32 = 1024 * 1024;

/// The largest SFTP data chunk either side takes.
pub const SFTP_CHUNK_MAX: u32 = 64 * 1024;

/// The most bytes one log read returns.
pub const LOG_READ_MAX: u32 = 64 * 1024;

/// An artifact's opaque identity, scoped to its worker's instance.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct ArtifactId(pub [u8; 16]);

impl ArtifactId {
    /// The artifact's lowercase hex.
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
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

/// An artifact's place in its lifecycle.
#[cereal::derived(Data, Copy, Eq)]
pub enum ArtifactState {
    Reserved,
    Published,
    Aborted,
    Expired,
    Evicted,
}

/// What a result or an attachment names an artifact by: its identity, size, and hash.
#[cereal::derived(Data, Copy, Eq)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// A reservation for an upload: its purpose, name, size, and hash, declared first.
#[cereal::derived(Data)]
pub struct ReserveUpload {
    pub purpose: ArtifactPurpose,
    /// Metadata alone, never a path component.
    pub display_name: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// A reservation granted: the identity, the staging path SFTP writes, its expiry.
#[cereal::derived(Data)]
pub struct UploadReservation {
    pub id: ArtifactId,
    pub staging_path: String,
    pub expires_at_ms: u64,
}

/// A published artifact as its worker describes it.
#[cereal::derived(Data)]
pub struct ArtifactDescription {
    pub id: ArtifactId,
    pub purpose: ArtifactPurpose,
    pub display_name: String,
    pub size: u64,
    pub sha256: [u8; 32],
    /// The execution host's path SFTP reads it from.
    pub sftp_path: String,
    pub state: ArtifactState,
}

/// Which of an operation's streams a log read takes.
#[cereal::derived(Data, Copy, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
}

/// A bounded read of a running operation's log.
#[cereal::derived(Data)]
pub struct ReadLog {
    pub operation: OperationId,
    pub stream: LogStream,
    pub offset: u64,
    /// At most `LOG_READ_MAX`.
    pub max_bytes: u32,
}

/// One log read's bytes and where they leave the reader.
#[cereal::derived(Data)]
pub struct LogChunk {
    pub generation: u64,
    pub offset: u64,
    pub bytes: Vec<u8>,
    /// The log holds nothing past these bytes yet.
    pub eof: bool,
    /// The operation ended; the log will grow no further.
    pub done: bool,
}

/// An artifact request.
#[cereal::derived(Data)]
pub enum ArtifactRequest {
    Reserve(ReserveUpload),
    Publish { id: ArtifactId },
    Abort { id: ArtifactId },
    Describe { id: ArtifactId },
    ReadLog(ReadLog),
}

/// An artifact request's answer.
#[cereal::derived(Data)]
pub enum ArtifactReply {
    Reserved(UploadReservation),
    Published(ArtifactDescription),
    Aborted { id: ArtifactId },
    Described(ArtifactDescription),
    Log(LogChunk),
}
