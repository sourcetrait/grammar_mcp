# oneshot.rs

## fn run_oneshot
Builds the same substrate the serve path builds, starts mcp's system of its own on it,
its engine alone, without the serve path's supervision, and a face of its own for the
Channel's commands and the engine's host asks, and dispatches one invocation straight
to the `pub(crate)` handler methods on `NuSh` - no rmcp transport anywhere.
That is what makes this the operator's no-agent contribution loop rather than a
parallel implementation to keep in step.

The invocation is its own host, its engine in its own process, so every tool it drives
names `local`: it takes no host of the operator's, since it reaches no other.

Deny does not apply on this path. It gates agent registration at router assembly, and
this bypasses the router entirely, so the operator always has the whole tool set; the
interface is built with no denied command for the same reason, so a command an agent's
host withholds still runs here.

The envelope's NUON text prints as it is. The exit codes are 0 for success, 1 for an
error envelope, an rmcp-boundary error, an equip substrate that cannot be built, or an
engine that cannot start, and 2, clap's own, for arguments it cannot parse. The data is
the engine's to read, as an agent's is, so text that isn't NUON answers an error envelope
like any other refusal. The envelope's `error` field is the only discriminator needed for
the first two.

Three caveats are by design rather than gaps: a `def --env` definition's stateful lane is
single-shot because the session state dies with the process; `interface process list` and
`interface process kill` reach the invocation's own engine, so its in-flight map is
empty; and writing into a namespace a live agent host is using is the operator's own risk -
git's index lock keeps the repo safe, but an in-flight agent call can transiently fail.
