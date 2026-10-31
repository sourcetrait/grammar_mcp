# run.rs

## fn run_main
The session's end is logged on stderr and the process exits 0; a failure is
logged behind `grammard: ` and exits by its code, so a script tells a held
namespace from a bad workdir without reading text. stdout carries nothing but
the wire.

## fn wire
The wire is stdout's own descriptor, duplicated, and fd 1 then points at
stderr, so nothing else in the process, a stray print or an external that
inherits fd 1, can write into the stream: what would corrupt a frame lands in
the MCP's log instead.

## fn serve
The environment is read once as data, an entry that isn't UTF-8 left out. The
start comes before the hello, so a worker answers the hello with its namespace
held, its engine up, and its scratch made, or with why it couldn't. The
shutdown runs whatever the session's outcome, so a session that never opened
still releases its lock and removes its scratch.

## fn refuse_with
A worker that couldn't start still reads the MCP's hello and refuses it with
its failure's kind and text, so the MCP reports a held namespace as
`daemon::unavailable` naming `runtime::namespace_held`, not as a lost session.
A runtime of its own is made for that, the worker's having never started.
