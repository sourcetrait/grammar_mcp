# admission.rs

## struct Admission
What every stream is held to, shared by the accept task's streams: the
worker's profile, the one lease table a hello binds in, and the budget of
streams at once.

### fn answer
The protocol's own handshake, `HelloState` over the lease table, so the worker
answers a hello exactly as the protocol's vectors were checked. A lease's id and
token come from one read of the OS's random source, drawn for every hello since
the protocol's `on_hello` takes them before it knows whether it grants; without
them no reply is sent, and the stream closes. The lease actions a binding
returns are left to ControllerLease, which keeps the streams they act on.

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
the same deadline, and the stream closes after it until Dispatch serves
admitted streams. The stream's place in the budget is held for the task's life.
