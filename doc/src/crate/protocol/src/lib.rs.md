# lib.rs

The crate is the portable side of the daemon split: typed data and the wire's
framing alone, with no evaluator, operator, MCP server, or platform monitoring
in its dependencies.
The engine, the client, the daemon, the MCP frontend, and the probe all take
it, and it takes none of them, so the package graph stays one-way.

## mod artifact
An artifact's metadata, the requests that reserve, publish, inspect, abort,
describe, and read a log, the store's bounds the hello advertises, and the
store a worker keeps of them; the bytes move by SFTP.

## mod channel
The Channel's reservations, caps, modes, and phases, and what crosses between a
frontend and the worker that owns the logical Channel: its state and epoch, the
frontend's notices, typed host events, and sends as data.

## mod codec
The wire's framing: the preface, the length-prefixed frames and their codec
over any byte stream, the pinned codec's identity, and the one door from a
payload to a value, a bounded decode.

## mod command
The `interface` tool's catalog, its commands, arguments, results, summaries, and
deny words, with both rendered blocks, shared by a frontend's description and
deny parsing and the runtime's dispatch.

## mod engine
The engine's requests, commands, and answers, with the diagnostics, their cap
and positions, the error, and the documents they carry, moved here from the
engine so the frontend and the daemon share one definition; the engine
re-exports them.

## mod error
The runtime's and the transport's errors, each rendering to the envelope's rows,
the transport's under stable `daemon::` kinds.

## mod event
The Events stream's cursors, records, gaps, resets, and acks, with the backlog
and the Channel gate a worker keeps.

## pub mod fixture
The v1 fixture driver, public as a module of its own, so a native test or the
probe decodes the golden vectors by type through it.

## mod hello
A stream's opening exchange, the negotiated bounds, and both ends' checks.

## mod lease
The controller lease's messages and intents, and the table a worker keeps.

## mod limits
The frame ceilings the wire contract sets, each stream role's.

## mod operation
An operation's ticket, admission, status, and outcome, and the table a worker
keeps before its journal makes them durable.

## mod operator
The operator's portable data, its manager's observation and the emergencies it
classifies, at fixed widths; the operator's Linux monitoring stays in its crate.

## mod peer
The retained remote Channel's models, its commands' typed results, and its
lifecycle and delivery events and relayed messages, as data a worker reports
to its frontend.

## mod runtime
The runtime's opaque identities, its request and reply, the host's metadata, the
purviews, and the controller's policy.

## mod stream
Each stream role's frames after its hello, both ways, with their ping and
close.

## use nuval::wide
The wire's widening, shared with nuval so every count and offset widens one
way.

## use sourcetrait_common::{cereal, datum, subsys}
Common comes through the workspace's `sourcetrait_common` and its facade, as
every crate takes it (User): cereal for the data, `datum`'s nonce and nom for a
request's identity and its host, and `subsys`'s error for a reply of the wrong
kind. `cereal::derived` emits absolute `::serde`, `::bitcode`, and `::rkyv`
paths, so those three are the crate's own dependencies.

## mod tk
## mod tku
tokio's byte-stream traits and tokio-util's codec and bytes, under the names
the mcp crate gives them, for the framing's adapter alone; the data needs
neither.

## pub use
The framing, its ceilings, its errors, and the schema's ID; the capability
set, which every side advertises; the interface catalog; the engine's protocol,
its requests, commands, and answers with their diagnostics, error, and
documents; the value bounds and their checks; and nuval's data under its own
names, the value, its error and result, and every kind, so one value and error
representation serves every endpoint without a second hierarchy.
