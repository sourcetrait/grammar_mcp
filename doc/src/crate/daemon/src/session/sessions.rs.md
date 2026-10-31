# sessions.rs

## type Clock
Given to the sessions rather than read inside them: the worker passes its
monotonic milliseconds since its start, a test its own hand, so a lease's
expiry is checked without waiting out the protocol's 30 s.

## struct Sessions
The protocol's `LeaseTable` and the registry of open streams under one lock,
so a lease's change and the streams it reaches are never out of step: a
takeover can't land between a hello's binding and its stream's registration.
Each move reads the clock inside the lock: a time read before it, by a request
that then waited on the lock, could be older than a move made meanwhile, and a
heartbeat acting on it would set the lease's deadline back or renew a lease
already past its expiry.

### fn admit
The hello's lease bound and the accepted stream registered in one hold of the
lock; a refused stream is never registered.

### fn request
The table answers a heartbeat or a release whatever stream carries it: the
lease and its token are what it checks.

### fn deregister
A stream's end, ordinary or not. The lease outlives it, to its release or its
expiry, so a controller can resume on a new stream.

### fn admits
Whether the lease is the one the table holds live, read from the table itself,
so a lost lease's id is never kept: `StopAdmission` needs no record, every
lease that loses its standing being one that is no longer live, and a set of
the stopped would grow by one id a lost lease for the worker's life.

### fn channel_epoch
What the table's `ResetChannel` records, for EventBroker to read once it
lands.

## fn ticker
Each second, so a lease lapses within a second of its deadline; a missed tick
is delayed rather than burst.

## fn Inner::carry
Every action at once, under the lock its change was made in. A loss's reason
comes from its `Notify` and goes into the `Close` its `CloseStreams` sends,
which is how an Events stream hears of the loss until EventBroker carries
notices as records.

## fn loss_error
Expiry as `LeaseExpired`; a release and a takeover as `LeaseInvalid`, each with
its reason, so the closed controller can tell which.
