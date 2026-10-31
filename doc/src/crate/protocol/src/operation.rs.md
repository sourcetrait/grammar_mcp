# operation.rs

The Operation contract's messages and an in-memory model of its admission,
which the daemon's journal will make durable. The model makes no exactly-once
claim: it runs an operation at most once, through one admission, and reports
truthfully what it knows of it afterwards.

## const TICKET_EXPIRY
## const TICKETS_MAX
A ticket waits 60 s for its submission, single-use, and at most 1024 wait at
once, so an issuing loop can't grow the table: past the bound a request for
a ticket is `Busy` until older ones expire.

## enum OperationState
The contract's seven states. `UnknownAfterRestart` is never collapsed into
`Failed` or `Cancelled`: the worker can't know whether an operation
interrupted by its stop had an effect.

## enum OperationStatus
`Expired` keeps the terminal state an operation reached where its tombstone
knows it, so a lookup after a result is let go still says how it ended.

## fn transition
Every legal move, and none other. A cancellation may be requested from
admitted, running, or already cancelling, the repeat a no-op; a completion
landing after it takes the one terminal, `Cancelled` only after the worker
saw the operation end; a restart turns every unfinished state unknown.

## fn named_command
The longest run of leading words that is a command's, `interface` itself
skipped, so a command's arguments never change its class.

## fn request_class
Exhaustive over every request, every engine command, and every catalog
command, with no default arm, so a new request or command fails the build
until it is classed. An `interface` text naming no command is held mutating:
nothing is retried on a guess. An artifact's inspection, description, and log
read are queries; a reservation, a publication, and an abort are mutating.

## fn payload_hash
Over the typed value re-encoded, never the frame's bytes. bitcode accepts
many byte forms of one value: a search of single-byte changes to a 70-byte
request found 332 that decode to the same request, each hashing alike, so no
ignored bit can make a second identity for one payload.

## struct OperationTicket
## struct Submission
## struct Admission
## struct StatusQuery
## struct StatusReply
## struct Response
The frame families' operation messages. Each carries the stream's
correlation apart from the operation's id, so a correlation, scoped to one
stream, never stands for an operation, and a caller whose reply was lost
still holds the id to look up.

## struct OperationTable
Tickets awaiting submission, and every operation admitted with its state and
its payload's hash.
### fn issue
An id already issued or admitted is `Busy`, as a full table is, so no ticket
ever names an operation twice.
### fn submit
An operation admitted before answers what it is, its result or its
tombstone, whatever its ticket's state, and a different payload under its id
conflicts. Its duplicate's hash is checked against the request it carries
first, as a fresh submission's is, so a submission claiming the first hash over
another body is refused as an invalid frame rather than associated with the
earlier operation, before its retirement, after it, and after a recovery
alike. A fresh submission needs its ticket known, unexpired, unused, this
generation's, this lease epoch's, the request's nonce the ticket's, and the
supplied hash the request's own; a refusal leaves the ticket for its own
submission. Only an admission returns an `Execute`.
### fn recover
A journal's records at a new generation, every unfinished one unknown, so a
restart never runs anything again.
### fn advance
A move the contract has no such transition for, or one of an unknown or
retired operation, answers none and leaves the state as it was.
### fn retire
A terminal result let go, a compact tombstone kept with its hash and its
terminal state, so the id never runs again and a lookup still answers. An
operation not yet terminal keeps its result, and the call answers false.
