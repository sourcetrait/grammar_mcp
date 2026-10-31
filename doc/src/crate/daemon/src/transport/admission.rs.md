# admission.rs

## struct Admission
What every stream is held to, shared by the accept task's streams: the
worker's profile, its sessions, and the budget of streams at once.

### fn answer
The protocol's own handshake, through the sessions, so the worker answers a
hello exactly as the protocol's vectors were checked and registers the stream
under its binding in the same step. A lease's id and token come from one read
of the OS's random source, drawn for every hello since the protocol's
`on_hello` takes them before it knows whether it grants; without them no reply
is sent, and the stream closes.

### fn hold
The budget and the peer's credentials, both settled in the accept loop before
anything is spawned: a stream past the budget, from another UID, root's among
them, or whose peer can't be read is closed there, unread, so a burst holds no
task and no descriptor past the budget.

## fn accept
A failed accept, a full descriptor table among its causes, pauses before the
next, so the loop never spins; the stop ends it.

## fn admit
An admitted stream's preface and one hello frame within the protocol's deadline,
the codec holding the frame to the hello's ceiling and the decode to its
budget; a stream that fails either is closed alone. The reply goes out within
the same deadline, and an accepted stream is served by its role from there
(`session/serve.rs`); one whose reply couldn't be sent is deregistered. The
stream's place in the budget is held for the task's life, so the budget counts
open streams, not only hellos.
