# journal.rs

## const CAP_DEFAULT
1 GiB of segments and sidecars before a fresh admission is refused, the
Operation contract's proposal. It bounds retention, so an operation in flight
may pass it.

## const INLINE_MAX
An outcome of up to 64 KiB of bitcode is kept in its entry, past that in a
sidecar, so a segment's line stays bounded for Recovery's scan. A parameter, to
be measured there.

## const OUTCOME_BUDGET
A kept outcome reads back within what any client could take: the hard heap,
and the wire's element and depth bounds.

## struct Journal
The model holds durable states alone. A fresh submission is checked without
committing, its entry synced, then admitted; each move is made only after its
entry is synced. An outcome is kept as its own bitcode, exact where a NUON
mapping isn't: a nonce's `u64` passes NUON's int, and a diagnostic's severity
isn't serialized. The journal holds no lease token and no request's body, its
hash standing for the body, so neither reaches an entry or a diagnostic.

### fn issue
A degraded journal issues nothing. The ids are drawn again while the model
holds either, the engine keying its calls by nonce, and refused after four
draws.

### fn submit
A duplicate of a finished operation is answered its kept outcome beside its
status. A fresh one is refused while the journal is degraded, while the cap is
reached, or while the filesystem's free space is short of the caller's
reservation beside those in flight, `daemon::storage_full`: nothing written,
nothing run. The reservation, the most bytes the operation's entries and
outcome may take, is the caller's, which knows the negotiated ceiling; the
free space keeps an admitted outcome from failing for room.

### fn advance
A repeated request of a cancellation writes nothing again.

### fn degrade
A failed write stops the journal: after a failed sync a file's state can't be
known, so nothing more is written or admitted, and each operation reads at its
last durable state, the one a restart finds. Said once, on stderr.

## fn wall_ms
The wall clock dates an entry for retention alone; a ticket's expiry runs on
the caller's monotonic `now_ms`.
