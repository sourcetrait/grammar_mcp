# msg.rs

The engine's own messages. The wire's requests, commands, and answers are the
protocol crate's, portable data every endpoint shares, and ride whole inside
these. What stays here needs the engine: the asks an eval makes of the process
hosting it and their answers, which the frozen wire never carries, and each
request naming its answer type for subsys against `EngineSystem`.

## enum ToEngine
## enum FromEngine
A subsys system takes one type each way, so the wire's pair rides one variant
and the host's another: down, a request or a host answer; up, an answer or a
host ask.

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
