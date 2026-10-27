# error.rs

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

## enum EngineError
Everything an eval reports is prose bound for `thread::returned_error`, with ONE
exception: a module-resolution cycle is a CONDITION an agent can route on, so it
reaches the envelope as its own kind rather than as a string that happens to read a
certain way. `Init` is the engine's start, which its host reports and exits on.

cereal data, so a refusal crosses as data where the engine is reached as a component;
mcp's `GrammarMcpError` takes each variant into the kind and message the wire has
always carried.

`From<String>` is what keeps every ordinary `?` site in the eval untouched by that
distinction.
