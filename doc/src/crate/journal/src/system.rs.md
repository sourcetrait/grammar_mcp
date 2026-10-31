# system.rs

## struct JournalSystem
The journal runs on a blocking task, lent to each request and given back with
its answer, so a sync never holds the runtime's threads, and the loop serves
one request at a time, the journal's one writer. One sync a step: grouping
several steps' lines under one sync is a change for a measurement to justify.

### fn init
The journal opens in the system's start, a failure carried whole in the start's
I/O error.

## fn draw_ids
Twenty-four bytes from `/dev/urandom` in one read: the operation's id and its
engine nonce.

## struct JournalHandle
### fn stop
The system halted, the journal dropped with it and its lock let go.
