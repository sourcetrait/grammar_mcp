# state.rs

The host's one Channel. Its modes, phases, reservations, and caps are the
protocol crate's, shared with the worker that will own the logical Channel.

## fn event_overflows / fn event_spill_pointer
The second, lower cap. `MAX_FRAME_BYTES` (1 MiB) is what one wss frame carries,
but the client truncates a task-notification at about 500 characters of the
whole rendered packet line - a claude-code constant the server never observes,
measured at 500. An event between the two transits and fires Sent (the
transport write), yet renders back to the agent truncated, silently and
unrecoverably. So a send whose rendered event would overflow the notification
cap spills: the full event NUON goes to a file and a compact
event_spill_pointer ({spilled_event_path, event_bytes}) rides the wire in its
place, so the agent reads <inbox>/<spilled_event_path> for the real event,
`event_bytes` its original size. event_overflows tests the rendered event's
character count against the cap minus a wrapper reserve (the id, from, model,
attached, and NUON punctuation around it), not the exact line, so one threshold
serves every send path without re-deriving the wrapper. Distinct from the frame
cap: an event over 1 MiB cannot ride a Deliver frame inline, so the remote spill
transfers it through the file connection under `EVENT_SPILL_DEST`, inside the
delivery's own inbox directory, which is why the spill is a send-path act, not a
render choice.

## enum PacketSink
One send path for both modes: `emit` and `send_control` push to whichever sink
the open installed, so the gate, the spam counter, the spill, and the render
never ask the mode.

## type CloseSignal
The completion half exists for shutdown. The frame is written by the hub task
asynchronously, so a caller that exits the process immediately after asking for
a close races that write and leaves the peer with exactly the bare 1006 the
explicit close exists to prevent. Awaiting an ack makes the ordering
deterministic instead of a sleep long enough to usually work.

## enum SpamVerdict
`notify` on `Stop` is true only on the first crossing of an episode: the refusal
has to persist for every later send, but announcing it every time would make the
report about spam into spam itself.

## struct OriginCounter
Both flags reset when the window empties and the origin is evicted, so a
producer that goes quiet and later misbehaves again is a fresh episode rather
than one that never ended.

## struct ChannelInner

### field counters
Bounded by eviction rather than by a cap: an origin whose window empties is
dropped, so this holds only currently-emitting work rather than one entry per
origin for the host's whole life.

### field close
The close signal does not share the packet queue. A close queued behind traffic
could arrive late or not at all, and without an explicit close frame the agent
sees a bare 1006 - indistinguishable from a crash.

### field verify_cancel
Holding the sender is what keeps the timer armed and dropping it is the cancel,
so verification, a close and a re-arm all stand the timer down. That is also why
re-opening cannot leave two timers racing one channel.

### field inbox
The directory each open makes (`scratch.rs`'s `make_channel_dir`), where both
producers meet: a channel's attachments and a remote link's landed files share
it, so a relayed packet's refs resolve for the local agent. A deliberate
`interface channel close` takes it and prunes it; every other teardown leaves
it. The shutdown close, the verify-timer expiry and the unverified-emit
teardown are all host-initiated, so an attachment stays readable by nonce when
the channel went away without the caller asking for it, until the host itself
ends.

## enum ChannelSendError
The variants are deliberately distinguishable: a background consumer must be
able to tell "stop working, the channel is gone" from "this one send failed", or
it will either exit on a blip or spin against a dead channel.

## struct ChannelHandle
Guarded by a std Mutex rather than an async one on purpose. The emit path runs
on the eval thread inside a synchronous nu `Command::run`, and
`UnboundedSender::send` is itself sync, so the whole path stays lock-cheap and
needs no runtime handle.

### fn new
Taking the starting policy rather than reading CONFIG is what makes the handle
constructible without a process-global, which is the only reason the state
machine and the counter can be unit-tested at all.

### fn take_inbox
Clearing is what makes it safe twice over: a second close cannot prune a
directory a later open recreated.

### fn install_sync
A sync channel binds nothing: its greeting is the first line queued, and its
claim flag starts unset, as a fresh hub's does, so the same verification gate
holds it.

### fn take_sync
Taking is the claim. A sync channel has no connection to claim it, so the first
sync, which hands the agent the greeting through the stdio session it drives,
sets the flag a hub's accept would; `interface channel ready` then verifies as
it does for a Monitor. A refused sync claims nothing.

### fn mark_verified
Verification requires a claim. The handshake asserts the agent owns the claiming
connection, so with no claim there is nothing to own. It also closes a real
hole: a verified-but-unclaimed channel would accept emits into a queue nothing
drains, because the receiver is still parked in the accept loop waiting for a
peer. The refusal names the mode, since what the agent does next differs: start
a Monitor, or sync.

### fn close_if_unverified
The phase is re-read here, under the lock, because verification can land between
the timer's sleep elapsing and its task being scheduled; a bare `close` would
then tear down a channel that had just proven itself.

### fn send_control
Bypasses the gate, and `emit` does not. That split is load-bearing rather than a
convenience: the gate keeps telemetry off an unverified peer, but the handshake
packet is the thing the peer verifies itself by seeing, so it necessarily
precedes verification.

### fn record_send_at
Counting keys on the origin, which is host-stamped, so a definition cannot
spread its traffic across identities to stay under the rate.

## static CHANNEL
A process-global rather than a threaded parameter. The channel is
one-per-process by design, exactly like CONFIG, and the alternative is passing
an `Arc<ChannelHandle>` through the server, dispatch, the eval entry points and
`register_nuapi` - five signatures on the hot eval path - for a value that can
never vary per eval. A server's Channel face reads the same handle rather than
owning its own (`server/channel/face.rs`), so the two can never diverge.

## fn close_locked
The teardown itself, with the lock already held, so the two entry points that
decide whether to close cannot drift about what closing does.

Dropping the shutdown sender signals the hub task even though nothing is ever
received on it. A sync channel has neither signal; dropping its sink drops its
queue.

## struct MsgId
Distinct from `Nonce` in name rather than in shape: nonces name evals throughout
this crate, so reusing the word on the wire would confuse two different things.

## fn mint_msg_id
The attached content is hashed, never its path - the path is derived from the
id, so hashing it would be circular. The generator mixes in a counter and a
timestamp, so two byte-identical packets still get distinct ids.

## fn render_nuon
Rendering from a Value is deterministic, which is what lets the id hash cover
the rendered form.

## fn escape_line
`to nuon` renders compactly but does not escape a newline inside a string value,
and the client batches frames arriving close together into one event joined by
newlines. So a literal newline in a packet is indistinguishable from a batch
boundary and a reader would see more records than were sent.

Safe because the only raw newlines a compact render can carry are inside
double-quoted strings, where `\n` and `\r` are the escapes nushell reads back -
the line still parses to the original value.
