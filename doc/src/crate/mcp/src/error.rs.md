# error.rs

ONE unified wire envelope for every tool-execution failure, built from typed Rust
values. No stringly-keyed disambiguation, no rendered-text middle layer, and no
JSON-RPC error codes for domain failures.

## enum GrammarMcpError
Stays the ergonomic typed value the impls build and bubble with `?`; it is not
serde-tagged on the wire. Two variants carry already-unified rows, `Engine` carries the
engine's error whole, its own violations' rows among them, and the rest are
single-condition. The rows are the engine's `Diagnostic`s (the engine's `error.rs`).

`From<io::Error>` blanket-converts to `Internal { phase: "io" }` so the tool impls can
use `?` freely rather than mapping at every filesystem call.

`From<EngineError>` wraps the engine's error whole: its kinds and messages are the
engine's, the equips' refusals and an eval's failure among them, as the wire has always
carried them.

### fn kind_str
The two violation variants `unreachable!()` here deliberately: they render through
`bucket`, and reaching this arm would mean a caller bypassed that path. `Engine` takes
the engine's kind.

`remote::open_failed` is the SYNCHRONOUS bind/connect failure of a blocking
`interface remote open` (ConnectionWoes): distinct from the async
`mcp/remote/Disconnected {error}` Channel model, which now fires only for a listener's
post-bind accept failure or an established link's later teardown.

An `interface` call's refusal is `interface::invalid` rows, gathered under
`InterfaceViolations`, each at its position in the command; a command that parsed and ran
refuses with its own kinds, as its tool did.

`remote::channel_unverified` refuses an `interface remote open` while the Channel isn't
verified, before any bind or connect: the link's notices and relays ride the Channel's
gate and would drop.

`host::unknown` is a tool's `host` alias naming no host this MCP knows, its message
naming `local`. The engine's own `host::unserved` stays apart: that is a request naming
a host its engine doesn't serve, which no tool sends.

## fn error_to_call_result
The failure envelope, `{nonce, error}`: the error's rows as a table of `kind`,
`source`, and `message`, rendered through the same seam as a success
(`envelope_text`, `server/tool/common.rs`). A violations error carries its error rows
alone and drops its warnings: a refusal's table holds what refused it.

It takes anything that becomes a `GrammarMcpError`, so an engine refusal reaches the
envelope with no wrap at the call.

WHY SUCCESS-SHAPE ERRORS: in the Claude Code CLI every `is_error = true` result and
every JSON-RPC error renders a red bullet with NO body, so a failure flagged as one is a
failure the agent cannot read. The envelope rides a plain success result, and its
`error` field is what tells the two shapes apart.

Every tool mints its nonce before any check, so every failure carries one. For an eval
it names the call's log directory, where a failure that produced output before dying
left its stdout and stderr.

`Err(mcp::ErrorData)` stays reserved for genuine rmcp-boundary failures, an envelope
that will not render among them.
