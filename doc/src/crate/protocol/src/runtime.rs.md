# runtime.rs

The runtime's identities. Each is 128 opaque bits the runtime or the client draws
from the operating system's CSPRNG; the protocol carries and prints them and never
makes one, so it needs no randomness of its own. A stream's correlation ids are
`u64` and scoped to that stream; none of these stands in for one.

## struct OperationId
An admitted operation's durable identity, which outlives the stream that asked for
it, so a caller whose transport failed can look the operation up rather than
guess whether it ran.

## struct PluginInfo
A tuple struct, so it serializes as the positional `[name, version]` pair rather
than a two-key object. Both slots are inferable from position, and an `Invalid`
registry item yields a `null` second slot rather than dropping it - the pair
keeps its arity whatever the plugin reported.

## struct PurviewRow
`namepath_patterns` is the DESIGN's own column name, so the persisted table reads
as it was specified rather than as an implementation happened to name it.

## struct InfoReply
`interface info`'s record as the execution host answers it: `tmp` and `transient`
are the worker's own scratch, never the frontend's, so a routed `info` names the
directories its definitions see.

## struct ControllerPolicy
The frontend's deny set, frozen when its controller attaches; the runtime refuses
a denied tool or command before dispatch. It is presentation policy, never a Unix
privilege boundary: it narrows what one frontend offers, and the account is still
the boundary.

## enum RuntimeRequest
Typed engine requests, an interface command's raw text, which the runtime parses
with its own catalog-built parser, the host's metadata, an operation's lookup,
the controller's policy, the frontend's Channel notices, artifact requests, and
a cancellation, admitted as an operation of its own and naming its target. The
operation's states and its lookup's answers live in operation.rs.
No variant carries a Unix identity, a worker identity, or a role: a request runs as
the worker it reaches, under the lease that admitted it.

## enum RuntimeReply
`Interface` carries the command's result as typed data the frontend renders as it
always has; `Refused` carries the runtime's refusal, which the frontend's envelope
renders through its rows. A remote Channel command answers `Remote` instead, its
result typed by its shape (peer.rs); it goes last, keeping the indices before it.
