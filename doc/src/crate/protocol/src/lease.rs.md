# lease.rs

The namespace's one controller lease, as messages and as a model the runtime
keeps one of. The model owns no clock and draws no randomness: the caller
passes its monotonic milliseconds and the fresh id and token a grant takes, so
every transition is deterministic and a test drives the boundaries exactly.

## const LEASE_HEARTBEAT
## const LEASE_EXPIRY
The Runtime contract's 10 s heartbeat and 30 s expiry. A grant carries both,
so the controller paces itself by the worker's numbers rather than its own.

## struct LeaseToken
Its Debug prints `LeaseToken(redacted)`, cereal's derive told `not(Debug)`, so
no derived Debug of anything holding one, an intent, a grant, a request, or
the table itself, puts a token in a log. `matches` compares in time the bytes
don't steer.

## struct LeaseEpoch
Raised at each grant, a claim's or a takeover's; a resume keeps it.

## enum LeaseIntent
The hello carries the intent, as the Wire contract puts the claim in the
controller's hello. `fits` is the role's allowance, RPC claiming, resuming,
or taking over, Events attaching, Status observing, and nothing else: the one
way a stream could reach past its role is an intent its role doesn't allow,
refused before anything binds. A resume and an attachment name the
generation they knew, so a token from an earlier worker is refused for what
it is.

## enum LeaseBinding
An attachment's binding names the lease and epoch, never the token back.

## enum LeaseRequest
## enum LeaseReply
## enum LeaseNotice
## enum LossReason
What crosses the RPC stream after the hello, and the loss the Events stream
is told of, with its reason.

## enum LeaseAction
What a change asks of the runtime: tell the lost lease's Events stream,
stop its admission, close its streams, or reset the Channel for a new epoch,
so a new controller inherits no verified Channel. No action names an
operation: losing a lease stops new admission and leaves what was admitted
to the Operation contract.

## struct Change
An outcome and the actions taken to reach it, apart, since an expiry found
on the way to a refusal still has its loss to act on.

## struct LeaseTable
### fn holder
The live lease a message names, its token matched in constant time. A wrong
token is `LeaseInvalid`; a lease that lapsed is `LeaseExpired`; one released
or taken over is `LeaseInvalid`, naming which; an id never granted is
`LeaseInvalid` too.
### fn tick
A lease lives while the clock is before its deadline and is lost at it.
### fn bind
Ticks first, so a hello after a lapse claims a free namespace. A claim
against a live lease is `Busy`; a takeover always wins, its predecessor lost
as taken over; a resume needs the live lease's id, token, epoch, and the
worker's generation, and renews it; an attachment needs the same and renews
nothing, the controller's heartbeats keeping the lease.
### fn request
A heartbeat renews from its own moment; one after the expiry finds the lease
lost and answers `LeaseExpired`. A message for a lease released or taken
over learns which, from the last loss the table keeps.
