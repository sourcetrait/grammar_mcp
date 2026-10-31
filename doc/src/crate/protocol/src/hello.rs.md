# hello.rs

A stream's opening exchange, the one place both ends decide whether a stream
may carry anything. The client's validation and the worker's are the same
functions, so a fixture run through both ends reaches one outcome: a stream
the worker readies is one the client accepts, and a mismatch one side refuses
the other refuses for the same reason.

Nothing in a hello is authority. The UID the worker checks on its socket and
the SSH session the client authenticates are the boundary; the hello's
instance, client id, and features are metadata a refusal names, so a forged
hello reaches no further than a wrong one.

## struct ProtocolVersion
## const PROTOCOL_VERSION
v1.0. The major must match exactly, and so must the schema ID, so a minor may
differ only beneath one layout: a minor that changed a layout would change the
schema and be refused as a schema mismatch.

## const HELLO_DEADLINE
## const AUTH_DEADLINE
The contract's finite phases, 5 s for a stream's hello and 30 s for the SSH
session's authentication. The state machine owns no timer: the transports
apply these.

## const HELLO_BUDGET
A hello decodes within 64 elements and its frame's 64 KiB of heap, far tighter
than a request's budget, since it arrives before anything is negotiated.

## struct Instance
An id and a namespace, each an ASCII letter or digit and up to 63 more of
those, `.`, `_`, or `-`, which leaves `.` and `..` out. A name outside that is
refused, never sanitized, so two names can never be folded into one identity.

### fn key
The contract's InstanceKey: the SHA-256 of each name's big-endian length and
bytes in turn. The lengths keep `ab` + `c` apart from `a` + `bc`.

## struct Limits
The bounds a hello negotiates, the contract's initial-limits table less what is
fixed: the hello's and the event frame's ceilings and the decoded elements and
depth. `least` takes each bound's minimum, `past` names the first bound above
another's, and `zero` the first at zero.

### fn frame_ceiling
RPC and Status take the negotiated RPC ceiling, the contract naming none of
Status's own; Events keep their fixed 1 MiB.

## const LIMITS_DEFAULT
## const LIMITS_HARD
The contract's defaults and hard ceilings. A worker configured past the hard
ones still grants none past them.

## struct ClientHello
## struct ServerHello
## enum HelloReply
The client's first frame, and the worker's: acceptance or a transport error,
after which the worker closes the stream. A required feature is a capability's
name as a string, so a later minor can add a feature without a layout change,
and a name the worker doesn't know is refused like one it has off.

## fn accept_hello
The worker's side: the version, schema, and codec; the expected instance valid
and the worker's own; every required feature on; no requested bound at zero;
and each effective bound the least of the request, the worker's, and the hard
ceilings.

## fn check_reply
The client's side of the same checks, and what only the client can check: the
worker's UID and instance its endpoint expects, the key derived from that
instance, the role echoed, no bound raised past the request, and the
generation against the last one seen. A new generation is reported in
`Session::new_generation`, so a caller drops what it held of the old worker, a
lease to resume or a verified Channel, rather than carrying it over.

## enum HelloState
One stream's handshake on the worker. `admit` refuses every request until a
hello readies the stream, so nothing dispatches before it; a second hello, the
only way a role could change, is refused and closes the stream; a refusal
closes it too. A reconnect is a new stream with a new state, so no handshake
outlives its stream. `Ready` boxes the accepted hello, some 200 bytes beside
two empty variants.
