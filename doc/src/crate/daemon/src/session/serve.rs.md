# serve.rs

## fn serve
The codec's ceiling is raised from the hello's to the role's negotiated one,
and each frame decoded within the negotiated budget. The stream is deregistered
however its loop ends.

## fn run
One loop for every role: a frame from the client, or the stream's stop rung by
its lease. A frame that doesn't decode closes the stream with its error; the
stop closes it with the lease's reason; the client's own end, or a failed
send, ends it at once.

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
