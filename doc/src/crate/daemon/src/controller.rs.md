# controller.rs

The worker as a collection of subsys systems under the one controller its
process owns (User), as the MCP is under its own: a system never calls another,
the controller starts each, routes between them, and stops them in order. The
journal is its first; the engine's and the operator's join as their tasks
bring them.

## const JOURNAL_DIR
The journal lives in the instance's durable state, `<state>/grammar/grammard/
<key>/operations/`, apart from the runtime home's socket and the disposable
cache.

## struct GrammardController
### fn start
The journal opens before the engine, so a worker that can't keep durable
operations never starts an engine to run them. Its lock held is
`runtime::journal_held`; its directory or files refused,
`runtime::journal_unavailable`.

### fn shutdown
Each system stopped and what it held let go: the journal's lock with its
system.

## struct JournalWatch
The journal sends nothing up unasked; an end no stop asked for is said on
stderr.
