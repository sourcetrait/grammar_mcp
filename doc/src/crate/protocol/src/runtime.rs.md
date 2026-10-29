# runtime.rs

The runtime's identities. Each is 128 opaque bits the runtime or the client draws
from the operating system's CSPRNG; the protocol carries and prints them and never
makes one, so it needs no randomness of its own. A stream's correlation ids are
`u64` and scoped to that stream; none of these stands in for one.

## struct OperationId
An admitted operation's durable identity, which outlives the stream that asked for
it, so a caller whose transport failed can look the operation up rather than
guess whether it ran.
