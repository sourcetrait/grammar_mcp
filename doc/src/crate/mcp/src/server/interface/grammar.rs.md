# grammar.rs

The `interface` tool's commands as data: one table the parse registers, the
description renders, and `--deny` names, so the three can never disagree. The
commands convert the tools that `execute`, `rerun`, and `call` left behind, each
keeping its tool's parameters and result as they were, `info` alone dropping its
purviews. The names are User's: `interface <topic> <action>`, or
`interface <action>` for `info` and `help`, with no underscore in any of them.

## enum ArgShape
Three shapes cover every argument the tools take: a string, a list of strings,
and a whole number. Anything richer would be a value the agent could build with
an expression, and the contract takes literals alone.

## struct CommandSpec
`result` is the result's kind alone, `record`, `table`, or `nothing`, as the
description shows it (User): the commands answer what their tools answered,
built by the same serde structs, so the shape is the envelope's to carry and no
second copy of it is kept here.

`summary` is a summary's length, 80 characters at most (User), which a unit test
holds; User's own summaries stand where his draft has them.

### fn words
The deny name: what follows `interface `. A command is denied by the words an
agent types after the head, never by an invented key.

### fn signature
Optional positionals stay optional in the parse; `config`'s thresholds are flags
because four optional positionals would make setting the last one mean passing
nulls for the rest. The category is cosmetic: the decl is never listed anywhere
but the description.

## const COMMANDS
Ordered as the description lists them, grouped by topic. A topic's commands sit
together because `render_signatures` opens a heading when the topic changes.

## struct InterfaceDecl
A signature for the parser and nothing more. The command is parsed, never
evaluated: dispatch is native, so `run` refuses, and reaching it would mean
something evaluated the interface engine, which nothing does.

## fn interface_engine
`EngineState::new()` holds no command at all, not even the language's keywords,
so the decls added here are the whole of what a command can name. That is the
isolation User asked for: no `use`, no `source`, no `let`, no external that
resolves to anything but a refusal. Built once at startup from the deny set,
which is fixed by then.

## fn render_signatures
Generated from the same table the parse registers, so the documentation can't
drift from what parses, and a denied command leaves both at once.

User's format: the note that each command begins with `interface `, then under
each topic's heading two lines a command, its summary and its signature in
backticks, with no bullet, no `def`, and the words after `interface` alone. It
keeps the description well inside Claude Code's 4,096 characters, which
`tool_docs` holds every description to.
