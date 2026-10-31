# serve.rs

## fn serve
The codec's ceiling is raised from the hello's to the role's negotiated one,
and each frame decoded within the negotiated budget. The stream is deregistered
however its loop ends.

## const CLOSE_DEADLINE
A close frame is a courtesy to a client that still reads: one that doesn't
would hold the stream, its registration, and its place in the budget for as
long as its buffer stays full, so the close waits 1 s and the stream is dropped
past it.

## fn run
One loop for every role: a frame from the client, or the stream's stop rung by
its lease. A frame that doesn't decode closes the stream with its error; the
stop closes it with the lease's reason; the client's own end, or a failed
send, ends it at once. An answer's send waits on the stop as the read does: a
client that sends and never reads fills the socket's buffer, and a send
waiting on it alone would keep the stream past its lease's loss. The send is
polled first and the stop only while it waits, since a release's own
`CloseStreams` rings the stop before its `Released` answer goes out, and that
answer is owed.

## fn rpc_turn
A lease message goes to the table, a ping comes back as a pong, and a call is
refused with `Unavailable` and the stream closed, since no call is served
before Dispatch; the lease stays.

## fn events_turn
A ping is answered. A subscription's resume and acks are held over for
EventBroker, which serves them.

## fn status_turn
A ping is answered. A lookup waits on OperationJournal and a health check on
OperatorLifecycle, each refused until then.
