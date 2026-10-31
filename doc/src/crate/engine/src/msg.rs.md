# msg.rs

The engine's own messages. The wire's requests, commands, and answers are the
protocol crate's, portable data every endpoint shares, and ride whole inside
these. What stays here needs the engine: the asks an eval makes of the process
hosting it and their answers, which the frozen wire never carries, and each
request naming its answer type for subsys against `EngineSystem`.

## enum ToEngine
## enum FromEngine
A subsys system takes one type each way, so the wire's pair rides one variant,
the engine's own requests another, and the host's a third: down, a request or a
host answer; up, an answer or a host ask. A request of the engine's own is local
to the process, never a new wire variant.

## enum PurviewRequest
The namespace's purviews as the engine keeps them beside its equip store: list
them, configure an id's patterns, resolve ids to what they put in view, and
adjust the rows for a rig installed or uninstalled (`equip/purview.rs`). Each
answers the runtime's error beside its value, the purview refusal and the
store's internal failures being the kinds those commands always answered.

## struct PurviewRigRequest
The session's view rides along, since a rig installed joins each purview it
has in view, and the view is its controller's.

## struct PurviewResolution
One read answers what a view needs: the patterns to match, expanded; each id's
own patterns, as `interface info` reports them; and the ids no purview answers
to.

## macro purview_request
Each purview request names its answer, so a controller gets its own type back.

## enum HostAsk
What an eval asks of the process hosting it. The Channel, the remote links,
the configuration, and the pins are the host's, and the engine reaches them
only by asking its controller. A body's send carries its data, the eval's stop
held back on its own side. The stop's report is an ask of its own, since the
action it reports is known only once the sender has stopped.

## enum HostAnswer
A message id, a reason in the host's words, the stop verdict, the record, or
the pin's or the report's acknowledgement. A decl puts a reason in its error's
label, where the envelope's message carries it.

## struct HostStop
The counts ride back to the sender with `notify`, so its report of the stop
carries what the error's emergency needs, the action beside them.

## macro wire_request
`ExecuteRequest`, `RerunRequest`, and `CallRequest` answer with a `ValueAnswer`,
`ControlRequest` with an `EngineResult<ControlAnswer>`, each wrapped as
`ToEngine::Wire`. The trait is subsys's and the request types are the
protocol's, so the impls stand here because `EngineSystem` and `ToEngine` are
this crate's: the orphan rule admits an impl whose trait parameter or target is
local.
