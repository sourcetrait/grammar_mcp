# oneshot.rs

## fn run_oneshot
Builds the SAME substrate the serve path builds and dispatches ONE invocation straight
to the `pub(crate)` handler methods on `NuSh` - no rmcp transport anywhere. That is
what makes this the operator's no-agent contribution loop rather than a parallel
implementation to keep in step.

DENY DOES NOT APPLY on this path. It gates agent REGISTRATION at router assembly, and
this bypasses the router entirely, so the operator always has the whole tool set.

The envelope's NUON text prints as it is. The exit codes are 0 for success, 1 for an
error envelope, an rmcp-boundary error, or an equip substrate that cannot be built, and
2 for unparseable operator input. The envelope's `error` field is the only discriminator
needed for the first two.

Three caveats are by design rather than gaps: a `def --env` definition's stateful lane is
single-shot because the session state dies with the process; `processes` and `kill` are
process-scoped, so a one-shot invocation's in-flight map is empty; and writing into a
namespace a live agent host is using is the operator's own risk - git's index lock keeps
the repo safe, but an in-flight agent call can transiently fail.

## fn nuon_record_arg
The parse chain is `from_nuon` to a nu Value, then `nu_json::Value::from_value`, then
serde_json: `CallParams` carries its args as the JSON object an agent's `call` sends, so
the operator's record crosses into the same shape.

`nu_json` rather than serde's own Serialize on a nu Value is load-bearing: the latter
emits the internal tagged representation with spans, not the friendly JSON `to json`
produces.

It exits 2 directly rather than returning a Result, because every failure here is
unparseable OPERATOR input and there is no envelope to put it in.
