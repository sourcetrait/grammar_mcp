# contract.rs

The `interface` tool's command held to its contract before anything runs: one
call of one interface command, unpiped, its arguments literals. User's two
rules are the contract's spine: the context has nothing else it can access, and
the agent can't pipe commands.

## enum InterfaceCommand
Each variant carries its tool's parameters struct as it stood, so the dispatch
calls the tool's own logic with the very value the tool used to deserialize.

## fn parse_command
nushell's own parser over the interface engine, so the arguments' types are
checked by the commands' signatures, as any Nu call's are, and an error lands
at the agent's line and column, measured from the command's start in the span
space (`file_start`), since a working set numbers spans after whatever the
engine holds.

A parse error is refused before anything else is read: a refused parse's
expressions are garbage. The rows are deduplicated and capped as a definition's
are.

## fn held_call
THE TWO RULES, made structural. One statement, one pipeline element, no
redirection: so no `;`, no newline-separated second command, no `|`, and no
`o>`. The element must be a call to an interface decl; with nothing else in the
engine, any other head parses as an external, which is refused here rather than
ever reaching a process.

## fn literal
The arguments are read off the AST, never evaluated: a string, a raw string, an
int, or a list of strings. A variable, `$nu` and `$env` among them, a cell path,
a subexpression, an interpolation, or a spread is refused, even where the
signature's type check would have let it through, because evaluating it is
access the contract withholds. `eval_constant` is deliberately not used: a
constant can read `$nu`.

## fn count
The tools took unsigned counts; a literal int is signed, so the range is checked
here, a refusal naming the flag.

## fn text / fn texts
A required argument's absence is the parse's to refuse, so these default rather
than fail.
