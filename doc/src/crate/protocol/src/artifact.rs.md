# artifact.rs

An artifact's metadata as a controller's requests describe, reserve, publish, and
inspect it. The bytes never ride an engine frame: SFTP moves them as the
authenticated Unix user, and these requests say what they are and where they go.

## const ARTIFACT_MAX_BYTES / ACTIVE_RESERVATIONS / STORE_QUOTA_BYTES / RESERVATION_EXPIRY_SECS
The defaults a worker advertises in its hello: an artifact's size, its unfinished
reservations, its store's quota, and an unfinished reservation's life. A worker may
lower them, or raise them within explicitly qualified ceilings.

## const SFTP_PACKET_MAX / SFTP_CHUNK_MAX
Held on both sides whatever a server advertises, so no remote length is trusted
before allocation.

## struct ArtifactRef
What an operation result or a Channel attachment names: its identity with its size
and hash, so the reader can check the bytes it fetched.

## struct ReserveUpload
The size and hash are declared before a byte moves, and publication requires them;
`display_name` is metadata alone, never a path component.

## struct ArtifactDescription
A published object's size, hash, and purpose never change; its `sftp_path` is a
POSIX string on the execution host, never expanded with the frontend's home.

## struct LogChunk
A running log is read in bounded steps: `eof` says no more bytes exist yet, `done`
that the operation ended and none will.
