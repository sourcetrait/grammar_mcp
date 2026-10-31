# operator.rs

The operator's side of mcp's system (`system.rs`): its handler, which routes its
emergencies and hears its end, and its observer, which feeds it the engine and the
Channel.

## struct OperatorWatch
The operator's emergencies arrive as packets no request waits on, so the control task
hands each to this handler, which puts it on the emergency lane beside the Channel's
spam pair; the one responder logs and announces both. Its status changes and its end go
to mcp's loop.

## fn engine_health
Bounded by one sample's interval: an engine that cannot answer within it gives that
observation no reading rather than stalling the observer, and the operator keeps its
active conditions across an absent reading.

## fn spawn_observer
One observation an interval, the operator's own cadence. The Channel's phase rides with
it because the Channel is mcp's: a `--test` host's operator samples only while it is
open. The phase is read from the running face's Channel (`server/channel/bridge.rs`),
and no face reads as closed. A send that fails means the operator's system is gone, and the observer ends with
it. Its handle is mcp's system's, which aborts it first when it shuts down.
