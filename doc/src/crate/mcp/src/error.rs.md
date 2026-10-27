# error.rs

ONE unified wire envelope for every tool-execution failure, built from typed Rust
values. No stringly-keyed disambiguation, no rendered-text middle layer, and no
JSON-RPC error codes for domain failures.

## struct Source
`path` is equip-repo-relative (`<equip>/<kind>/<file>`) for a validation diagnostic
and NULL for an `execute` definition's diagnostic, because that one is located in the
definition and there is no file to name. `position` is 1-based, and `[0, 0]` means
file-level rather than line 0.

Both fields serialize as PRESENT, with `path` as null when absent rather than omitted.
The whole `Source` is null on the `Diagnostic` for a non-located condition - an eval
timeout, an unregistered-equip error.

The shape stays flat - `path` plus `position` - which is what the agent consumes
directly, rather than a nested source-carrier the reader would have to unwrap.

## struct Diagnostic
The single type for both a validation finding and a body-lint finding - one shape
the wire and the check summary both carry.

WHERE A ROW RIDES CONVEYS ITS SEVERITY, which is why `severity` is `#[serde(skip)]` and
absent from the wire: an error row in a failure's `error` table, a warning row in the
`warnings` of the result that raised it, an `interface equip` commit's, install's, or
check's. It is an internal partition key only.

Per-condition data (`timeout_ms`, names, reason) lives IN `message`, not a typed
per-variant field: it is text either way, and a per-variant field would vary its shape
by kind.

## enum GrammarMcpError
Stays the ergonomic typed value the impls build and bubble with `?`; it is not
serde-tagged on the wire. Three variants carry already-unified rows, and the rest are
single-condition.

`From<io::Error>` blanket-converts to `Internal { phase: "io" }` so the equip impls can
use `?` freely rather than mapping at every filesystem call.

### fn kind_str
The three violation variants `unreachable!()` here deliberately: they render through
`bucket`, and reaching this arm would mean a caller bypassed that path.

REGISTRATION is `equip::*`, because the unit installed, committed, and addressed by a
source path is the equip; a rig's tree rules keep `rig::*` and a lib's are `lib::*`.
`equip::no_rig` stays apart from `equip::not_registered`: the one is an installed equip
asked for calls it does not have, the other a name nothing answers to.

`module::circular_import` is a TOP-LEVEL namespace rather than a `rig::` kind, because
a resolution cycle is not a rig-structure rule even when the validator is what finds
it. The validator already emits non-`rig::` kinds, `lint::summary_length` among them.

`remote::open_failed` is the SYNCHRONOUS bind/connect failure of a blocking
`interface remote open` (ConnectionWoes): distinct from the async
`mcp/remote/Disconnected {error}` Channel model, which now fires only for a listener's
post-bind accept failure or an established link's later teardown.

An `execute` call's refusals each name their part of the contract: `definition::invalid` rows,
gathered with the lint's under `DefinitionViolations`, for the definition; `data::invalid`
for data that isn't NUON or doesn't conform; `timeout::invalid` for a timeout that isn't a
positive duration; and `result::invalid` for a value the definition's return type refuses.
Each message carries the path or the position, so the agent fixes the one thing named.

An `interface` call's refusal is `interface::invalid` rows, gathered under
`InterfaceViolations`, each at its position in the command; a command that parsed and ran
refuses with its own kinds, as its tool did.

`remote::channel_unverified` refuses an `interface remote open` while the Channel isn't
verified, before any bind or connect: the link's notices and relays ride the Channel's
gate and would drop.

### fn message
An equip is named as every other surface names it, through `display_equip`: a builtin
by its bare name, `grammar:eye` rather than the `sourcetrait/grammar:eye` the namepath
resolved to, which no agent spells.

## fn error_to_call_result
The failure envelope, `{nonce, error}`: the error's rows as a table of `kind`,
`source`, and `message`, rendered through the same seam as a success
(`envelope_text`, `server/tool/common.rs`). A violations error carries its error rows
alone and drops its warnings: a refusal's table holds what refused it.

WHY SUCCESS-SHAPE ERRORS: in the Claude Code CLI every `is_error = true` result and
every JSON-RPC error renders a red bullet with NO body, so a failure flagged as one is a
failure the agent cannot read. The envelope rides a plain success result, and its
`error` field is what tells the two shapes apart.

Every tool mints its nonce before any check, so every failure carries one. For an eval
it names the call's log directory, where a failure that produced output before dying
left its stdout and stderr.

`Err(mcp::ErrorData)` stays reserved for genuine rmcp-boundary failures, an envelope
that will not render among them.
