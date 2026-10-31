# artifact.rs

The Artifact contract's metadata: what reserves, publishes, inspects, aborts,
and describes the objects SFTP carries, the bounded read of a running
operation's log, and the store a worker keeps of them. No artifact request or
reply carries bulk bytes. A log chunk of at most 64 KiB is the one byte
payload, and a growing log is never described as an artifact with a digest.

The store owns no clock and draws no randomness. The caller passes the wall
clock's milliseconds, since a record's deadline outlives its process in the
journal, and the fresh id a reservation takes. Every change answers the
actions it asks of the runtime, which journals the change before taking them.

## const ARTIFACT_MAX_BYTES
## const ACTIVE_RESERVATIONS
## const STORE_QUOTA_BYTES
## const RESERVATION_EXPIRY_SECS
## const RESULT_RETENTION_SECS
The contract's defaults: 256 MiB an artifact, eight unfinished reservations, a
4 GiB store, a one-hour reservation, and the Operation contract's seven-day
result retention, which every published artifact keeps at least.

## const SFTP_PACKET_MAX
## const SFTP_CHUNK_MAX
## const LOG_READ_MAX
Fixed, never advertised: 1 MiB packets and 64 KiB data chunks either side
takes, whatever a server advertises, so no remote length is trusted before
allocation, and a 64 KiB log read.

## const EMPTY_SHA256
`e3b0c442...b855`, held to sha2's digest of no bytes by the contract test.

## struct ArtifactLimits
## const ARTIFACT_LIMITS_DEFAULT
Advertised in the server's hello and never negotiated: the store is one every
lease shares, so a client asking for a smaller one would mean nothing. The
contract makes the bounds configurable downward, or upward within explicitly
qualified ceilings. None above the defaults is qualified yet, so each default
is its ceiling. The retention is configurable upward alone, so the result
window is its floor.
### fn out_of_contract
The first bound at zero or past its ceiling, a store smaller than one
artifact, or a retention short of the window. The client refuses a hello that
advertises one (hello.rs), and a worker's configuration is held to the same
check.

## struct RemotePath
A path on the execution host, kept as the host spells it: POSIX, absolute,
each component plain, no NUL, within 4096 bytes. It has no conversion to a
local `Path`, so the compiler refuses a remote path used as a frontend file,
and no client OS rule reinterprets one: `/srv/a\b` is one component on the
host, never a Windows separator. A decoded path bypasses `new`, so a client
checks one it receives before SFTP opens it.

## struct ArtifactId
Opaque, 128 random bits the runtime draws for each reservation, scoped to its
worker's instance. Its hex names the store's files. `from_hex` reads a name
back at recovery, and a name that isn't 32 lowercase hex digits is no id.

## enum ArtifactPurpose
### fn uploadable
A client reserves an equip's source alone. Results, logs, and received
attachments are the worker's to publish, through `publish_local`. Uploading
anything else would take a new variant, so nothing is uploaded under a
purpose the worker reads as its own.

## enum ArtifactState
`Publishing` is a state of its own, so an inspection during the copy answers
truthfully and no abort or second publish races the copy. `Expired` and
`Evicted` stay as tombstones, so a lookup after either says so rather than
answering unknown.

## struct ArtifactRef
What an operation result or a Channel attachment names: the identity with its
size and digest, so a reader can check the bytes it fetched.

## fn check_display_name
Metadata alone, never a path component: 1 to 255 bytes and no control
character, so a name an agent reads can't carry a terminal escape. A name
that looks like a path, a UID, or a namespace is kept verbatim and selects
nothing.

## fn check_digest
A digest of zeros is refused as a placeholder, and an empty artifact must
declare the empty digest, so every declared digest is one a copy can match.

## fn check_dest
An attachment's destination within its delivery, which the frontend joins
beneath its own inbox on whatever OS it runs. The rules are the union every
client OS needs: relative and `/`-separated; no empty, `.`, or `..`
component; no component past 255 bytes; none of `\ : * ? " < > |` or a
control character; no trailing dot or space, which Windows strips; and no
Windows device name as a component's stem in any case, `CON.txt` and
`aux .txt` among them. The mcp crate's `safe_dest` holds a dest to the host's
own path rules, and on Linux those take `..\..\x` as one name. This check keeps
such a dest out of a Windows frontend's parent directories.

## struct ReserveUpload
The size and digest are declared before a byte moves, and publication requires
them.
### fn check
The purpose an upload's, the name and digest held to their rules, and the size
within one artifact's bound. The store then holds the count and the bytes.

## struct UploadReservation
`expires_ms` is relative, as a ticket's is: a deadline on the worker's clock
means nothing on the client's.

## struct PublishUpload
## struct InspectArtifact
## struct AbortUpload
## struct DescribeArtifact
The contract's requests, each naming an id alone, so no request selects a
path, a UID, or a namespace. Inspection is how a client resolves an
acknowledgement it lost, never by uploading again or assuming the publication
failed.

## struct ArtifactDescription
A published object's reference, its size and digest immutable, its purpose,
and the exact SFTP path, never expanded with the frontend's home. Only a
published object has a description.

## struct ReadLog
### fn window
A read of no bytes or past 64 KiB is refused, and one past the log's end is
refused rather than reset, as an event cursor past the newest is. At the end,
a read takes nothing and reports `eof`.

## struct LogExtent
The runtime takes `done` before `len`. A log whose operation had ended when
its length was taken holds every byte it will.

## struct LogChunk
`generation` is the worker generation whose operation wrote the log, so a
reader can tell a log from before a restart, which is final. `eof` says the
read reached the log's end for now, `done` that the operation ended, and a
reader stops at both.

## enum ArtifactError
A refusal changes nothing. `Foreign` keeps another lease's reservation apart
from an unknown id, and each names only the id it was asked. RuntimeError
carries one as `Artifact`, so the frontend renders it as one row under its
`artifact::` kind.

## enum ArtifactHold
## enum ArtifactAction
## struct PublishPlan
The contract's copy: the staging file opened without following a link, its
type and size checked, its bytes copied into a new O_EXCL inode while hashed,
the copy synced and linked into `objects/`, and the directory synced. The
SFTP-written inode is never renamed into place, since a remote writer could
still hold it open.

## struct ArtifactStore
### fn reserve
The id is the caller's, drawn from the OS's CSPRNG, and one already held is
refused. The count and the bytes are taken as of `now_ms`, so a lapsed
reservation holds neither before `collect` makes it a tombstone.
### fn begin_publish
A publish of a published object answers it again, whoever asks: the contract's
repeated publication. A reservation is published by its own lease alone.
### fn finish_publish
Published only where the copy is the declared size and digest. A short,
changed, or failed copy leaves the reservation and its staging file for a
retry while its lease holds it, and aborts it once no lease does. The staging
name is removed only after the publication.
### fn abort
Repeated, it answers the same. A published object is never retracted, and a
copy under way is never aborted from under itself.
### fn release
A lease owns its reservations. Its loss, whether it lapsed, was released, or
was taken over, abandons every unfinished one and frees its quota. A
publication it began finishes unowned: published if its copy verifies, aborted
if not, with no lease left to retry it.
### fn collect
A published artifact is kept through the retention window and for as long as
anything holds it: a retained result, a pending event's manifest, or an open
reader. Collection leaves a tombstone, so a lookup answers `Evicted`. A
recovered store evicts nothing until its holds are restored
(`holds_restored`).

### fn holds_restored
A start's holds are not the journal's: a recovered store comes back with none,
and a retained result or a pending event can outlive the retention window. So
recovery opens a phase in which collection evicts nothing, and the runtime ends
it once it has taken every durable reference's hold again; an artifact nothing
durable holds is collected after that as any other.
### fn publish_local
The worker's own objects, written into `objects/` and measured before they are
published. A refusal leaves the runtime to remove the file it wrote.
### fn recover
The contract's reconciliation at a start. An unfinished record is adopted only
where its object measures the reservation's own size and digest, its metadata
commit interrupted. Otherwise it is aborted, since its lease ended with the old
process. Only those objects are measured, so a start hashes nothing already
published. A published record whose object is gone reads `Evicted`. A staging
file is removed where its record ended and quarantined where no record names
it. An object no published record names, a partial copy among them, is
quarantined, never promoted by its name alone, and an evicted record's leftover
object is removed. A listed name that isn't one plain component is left alone.
