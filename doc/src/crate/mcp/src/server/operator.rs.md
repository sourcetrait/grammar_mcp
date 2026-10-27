# operator.rs

mcp as the operator's manager until mcp is a subsys of its own: the operator started on
the namespace, the engine and the Channel observed for it, and its emergencies put on
the host's lane.

## struct EmergencyRouter
The operator's emergencies arrive as packets no request waits on, so the control task
hands each to this handler, which puts it on the emergency lane beside the Channel's
spam pair; the one responder logs and announces both.

## fn start_operator
The operator's parameters are the config's: its lines, the `--test` flag, and the
process's `mcp_nom`, which its host lock carries. Its paths are the namespace's homes,
the lock under the data home, where it always was.

## fn engine_reading
The engine and the operator know nothing of each other, so the engine's health is
mapped here into the operator's reading, lane for lane.

## fn engine_health
Bounded by one sample's interval: an engine that cannot answer within it gives that
observation no reading rather than stalling the observer, and the operator keeps its
active conditions across an absent reading.

## fn spawn_observer
One observation an interval, the operator's own cadence. The Channel's phase rides with
it because the Channel is mcp's: a `--test` host's operator samples only while it is
open. A send that fails means the operator's system is gone, and the observer ends with
it.
