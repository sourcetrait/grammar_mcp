# event.rs

The Events stream's contract and the model a worker keeps of it. Loss is
explicit and bounded: a subscriber learns of every event it missed, as a gap
or a generation's reset, and never has its cursor reset quietly.

## struct EventCursor
A generation and the last sequence held contiguously. Sequences start at 1
each generation, so a cursor's sequence of 0 holds nothing.

## enum EventPayload
## struct EventArtifact
## struct EventRecord
A record names the Channel epoch it was emitted under, so events left
pending across a new lease's reset still say which Channel saw them. Its
artifacts are references, an id, size, and hash, with the destination within
the delivery: the frontend fetches and verifies the bytes and shows its own
inbox path, and no worker-relative path is ever read as a frontend file.

## enum EventFrame
`Record` boxes its record, which carries a host event's value, against the
small `Gap` and `GenerationReset`. A gap's range is inclusive.

## struct EventResume
## struct EventAck
An ack moves the subscriber's receipt cursor and nothing else: receipt is not
an agent having processed the event, and no operation's state moves with it.

## struct EventBacklog
Held to the hello's `event_records` and `event_bytes`, the oldest dropped past
either, apart from the frontend's 1000-packet sync queue. A record past the
1 MiB event frame is refused when pushed, numbering nothing, and so is one
whose attachment destination fails `check_dest` (artifact.rs): no event is
numbered that a frontend on any OS couldn't materialize beneath its inbox.
### fn read
A cursor of another generation reads a reset and then the new generation from
its oldest; one whose next sequence was dropped reads a gap first; one past
the newest is refused, never reset.
### fn deliver
A subscriber more than its window behind is closed rather than held, so a
slow reader never holds the engine's control capacity; within its window it
reads a gap and goes on.
### fn ack
Receipt never moves backwards and never past the newest event, and an ack of
another generation's events is refused.

## struct ChannelGate
The Channel's epoch and phase on the worker. An open must name a newer epoch;
a claim and a ready must name the current one, in order; a new lease's reset
closes the Channel, so a verification never crosses epochs and a new
controller claims and readies afresh.
