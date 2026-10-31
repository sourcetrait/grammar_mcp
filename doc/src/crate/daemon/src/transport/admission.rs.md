# admission.rs

## struct Admission
What every stream is held to, shared by the accept task's streams: the
worker's profile, the one lease table a hello binds in, and the budget of
streams at once.

### fn answer
The protocol's own handshake, `HelloState` over the lease table, so the worker
answers a hello exactly as the protocol's vectors were checked. A lease's id and
token come from the OS; without them no reply is sent, and the stream closes.
The lease actions a binding returns are left to ControllerLease, which keeps the
streams they act on.

## fn accept
A failed accept, a full descriptor table among its causes, pauses before the
next, so the loop never spins; the stop ends it.

## fn admit
The order is the admission: the budget first, so a stream past it costs
nothing; then the peer's credentials, read from the kernel before a byte of the
stream, a UID other than the worker's closing it unread with no exception for
root; then the preface and one hello frame within the protocol's deadline, the
codec holding the frame to the hello's ceiling and the decode to its budget. A
stream that fails any step is closed alone. The reply goes out within the same
deadline, and the stream closes after it until Dispatch serves admitted
streams.
