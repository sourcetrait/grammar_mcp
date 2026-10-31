# file.rs

## const LOCK_FILE
The journal's own lock beside its files: two instance files of one id and
namespace with different data homes take different namespace locks and can
share one state home, so the namespace lock alone wouldn't keep a second
writer out.

## const SEGMENTS
## const RESULTS
A segment a start, and the outcomes past the inline bound, each in a directory
of its own, so a scan counts each kind's bytes and a stray name never reads as
a segment.

## enum Fault
## struct Faults
Each write, sync, rename, and directory sync is a point a plan fails at a hit
it counts, from one: a test reaches the boundary between any two durable
states. A product runs the empty plan.

## struct JournalDir
### fn open
The root and its two directories made where missing and refused unless each is
a plain directory the UID owns that opens to no one else. The lock is taken
without waiting and never removed or replaced. Every regular file's bytes count
against the cap, an earlier start's segments and sidecars among them.

## fn make_dirs
Each missing directory made 0700 and synced into its parent, so a fresh chain
of them survives a crash with the segment that depends on it.

## fn segment_number
`<n>.nuonl`, `n` plain decimal, so a stray name never raises the next number.

## fn sync_dir
A file's name is durable only once its directory is synced: a segment's and a
sidecar's before any entry rests on it.

## struct Segment
### fn create
Made exclusively and never reopened: each start writes its own, so no start
writes after another's torn end, and only Recovery reads an earlier one.
### fn append
Written and synced before it returns, the order every answer rests on.
### fn read
A kept outcome's line read back by its place, its checksum held.

## fn put_sidecar
An outcome past the inline bound, one NUON binary value named by its SHA-256:
written under a temporary name, synced, renamed, and its directory synced,
before the entry that names it is written. A name already there is the same
bytes, kept once. A temporary name a start that died left is this journal's
own, under its lock, and is replaced.

## fn read_sidecar
Its bytes held to its name's hash, so a damaged sidecar is never an outcome.
