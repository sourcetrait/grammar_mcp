# sessions.rs

## type Clock
Given to the sessions rather than read inside them: the worker passes its
monotonic milliseconds since its start, a test its own hand, so a lease's
expiry is checked without waiting out the protocol's 30 s.

## struct Sessions
The protocol's `LeaseTable` and the registry of open streams under one lock,
so a lease's change and the streams it reaches are never out of step: a
takeover can't land between a hello's binding and its stream's registration.

### fn admit
The hello's lease bound and the accepted stream registered in one hold of the
lock; a refused stream is never registered.

### fn request
The table answers a heartbeat or a release whatever stream carries it: the
lease and its token are what it checks.

### fn deregister
A stream's end, ordinary or not. The lease outlives it, to its release or its
expiry, so a controller can resume on a new stream.

### fn admission_stopped, fn channel_epoch
What the table's `StopAdmission` and `ResetChannel` record, for Dispatch and
EventBroker to read once they land.

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
