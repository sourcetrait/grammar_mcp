# nu.rs

The `nu` tool, NuNu's execute: one definition evaluated against NUON data. The contract
is User's `doc/human/NuNu.md`.

## struct NuParams
Three strings, nothing structured: the definition is Nu source, the data NUON text, the
timeout a Nu duration's text. The agent writes Nu and NUON, never a JSON schema; the
definition's own signature is the schema.

## fn parse_timeout
The tool's `timeout` is the call's timeout and nothing else: parsed as NUON, which
reads a duration literal, and held to a positive duration. It is converted to whole
milliseconds rounded up, so a sub-millisecond duration still gets one. Its value goes
on to the definition's `timeout?` when the signature takes one, so a body can budget
against what the host allows it. 120sec when absent, uncapped as every tool's timeout
is.

## fn prepare_nu
THE ORDER IS THE CONTRACT'S: the definition first, since its signature is what the data
is checked against; then the data; then the timeout. Every refusal comes before any log
directory exists.

The data is parsed by `from_nuon`, which evaluates constants alone and refuses a
pipeline, then held to the declared type by the strict check, then rendered back by
`to_nuon` as the call's literal. The literal is therefore the host's own render of a
plain value, never the agent's text: nothing the agent wrote reaches the source but the
definition, which the contract has already held to one def. An empty text is refused
rather than read as null, since a `data: nothing` definition takes `null` and the field
is never optional.

A rerun passes `lint` false: its definition is the host's own cache of a definition the
lint already passed.

## fn dispatch_nu
The lane is the definition's: `def execute` on the stateless pool, `def --env execute` on
the stateful thread, so the agent picks persistence in the language it writes rather
than by choosing a tool. Both dispatches cache the definition beside the call's logs
before the eval, so a call that timed out can still be rerun with a larger timeout.

THE RESULT CHECK runs on the eval's value before anything renders it: nushell checks no
output when a command runs, so a body declaring `record<a: int>` and returning a list, a
record with an extra field, or nothing at all would otherwise succeed. A mismatch is
`result::invalid` with the nonce; the eval's side effects stand, nothing being rolled back.

A value that passes is the envelope's `result` itself, rendered once as NUON with no
JSON conversion, so a duration stays a duration and a filesize a filesize.

## fn nu
The description is User's `include/human/mcp/execute.md`, verbatim through
`include_str!`: rmcp's tool macro takes a doc attribute as the description when no
`description` is given, a macro call included, so the agent reads his contract as he
wrote it. The doc lint's list-indent rule is allowed on this one fn: the text is written
for the agent, not for rustdoc, and it stays as he wrote it.

The nonce is minted FIRST, over the parameters, so every refusal carries it.
