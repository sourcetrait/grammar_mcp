# stream.rs

What each stream role carries after its hello, as one type a direction, so a
stream reads a frame as its role's type and no other. The role is fixed by the
hello and never changes, so a frame of another role's type never reaches a
stream's reader.

## struct Ping
The Wire contract's bounded liveness: the answer echoes the nonce, so a probe
matches its answer however frames interleave.

## struct StreamClose
An orderly detach, never a worker's shutdown: a client's close ends its stream
alone, and only a lease's release ends its lease. A worker's close names why
where it wasn't asked: a lease lost, or a subscriber fallen past its window.

## enum RpcFrame / enum RpcAnswer
The controller's families: a ticket, a submission under it, a status lookup,
the lease's heartbeat and release, and the answers to each, every one carrying
its stream's correlation apart from any operation. A lease message's answer
carries its refusal, since `LeaseReply` has none of its own. The submission and
the response are boxed, each the size of its request or result.

## enum EventsFrame / enum EventsAnswer
A subscription's resume, its first frame, and its acks; the worker's events,
gaps, and resets.

## enum StatusFrame / enum StatusAnswer
The read-only role: an operation's lookup and the worker's health, as the
operator observes it, and nothing that executes. Identity is the hello's.
