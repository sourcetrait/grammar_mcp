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

`position` is a pair of u64s, the wire's fixed width, built from the parser's
usize line and column through nuval's `wide`, which never alters one: the same
diagnostic reads the same from any platform's worker.

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
mcp's `GrammarMcpError` carries it whole, its kind and message the engine's, as the
wire has always carried them.

The equips' refusals are the engine's too. REGISTRATION is `equip::*`, because the unit
installed, committed, and addressed by a source path is the equip; a rig's tree rules
keep `rig::*` and a lib's are `lib::*`. `equip::no_rig` stays apart from
`equip::not_registered`: the one is an installed equip asked for calls it does not
have, the other a name nothing answers to.

An `execute` call's refusals each name their part of the contract: `definition::invalid`
rows, gathered with the lint's under `DefinitionViolations`, for the definition;
`data::invalid` for data that isn't NUON or doesn't conform; `timeout::invalid` for a
timeout that isn't a positive duration; and `result::invalid` for a value the definition's
return type refuses. Each message carries the path or the position, so the agent fixes the
one thing named.

`host::unserved` refuses a request naming a host whose engine this process doesn't serve,
before anything of it runs.

`module::circular_import` is a TOP-LEVEL namespace rather than a `rig::` kind, because
a resolution cycle is not a rig-structure rule even when the validator is what finds
it. The validator already emits non-`rig::` kinds, `lint::summary_length` among them.

`From<String>` is what keeps every ordinary `?` site in the eval untouched by that
distinction, and `From<io::Error>`, `Internal { phase: "io" }`, every filesystem call
in the equips' lifecycle.

### fn message
An equip is named as every other surface names it, through `display_equip`: a builtin
by its bare name, `grammar:eye` rather than the `sourcetrait/grammar:eye` the namepath
resolved to, which no agent spells. `Init` reads as its reason alone, the startup
message the host prints.

### fn rows
A violations error's rows are its errors alone, its warnings dropped: a refusal's table
holds what refused it. `kind` and `message` serve a single-condition error and never a
violations error, which renders through its rows; `Display` joins those rows' messages.
