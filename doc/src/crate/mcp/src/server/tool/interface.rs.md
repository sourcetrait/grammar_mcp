# interface.rs

The `interface` tool: every tool but `nu`, `rerun`, and `call`, folded into one
`command` string (User's NuNu, `doc/human/NuNu.md`'s `# TOOL: grammar/interface`).
The commands convert those tools as they were: same parameters, same results,
same refusals.

## const INTERFACE_PRETEXT
User's `include/human/mcp/interface.md`, verbatim by `include_str!`: his pretext
and nothing else, the signatures being the code's to render.

## fn interface_description
His pretext, its trailing whitespace trimmed so one blank line parts it from the
signatures whatever his file ends with, then the command signatures for the deny
set, rendered from the table the parse registers (`server/interface/grammar.rs`),
so the documentation is never a second copy to keep in step.

## fn interface
The nonce is minted FIRST, over the tool's name and the command text, so a
refusal at the parse carries one exactly as a dispatched command's answer does.
The dispatched command answers under that same nonce: each command's method
takes the nonce rather than minting its own.

## fn run_interface
One `match`, each arm a command's method. No command dispatches an eval, so
none appears in `interface process list`'s table and none takes a lane or a
permit; a command's own locks are taken inside its method as its tool took them.
