# lib.rs

The crate is the portable side of the daemon split: typed data alone, with no
evaluator, operator, MCP server, or platform monitoring in its dependencies.
The engine, the client, the daemon, the MCP frontend, and the probe all take
it, and it takes none of them, so the package graph stays one-way.

## mod engine
The engine's requests, commands, and answers, with the diagnostics, the error,
and the documents they carry, moved here from the engine so the frontend and
the daemon share one definition; the engine re-exports them.

## use nuval::wide
The wire's widening, shared with nuval so every count and offset widens one
way.

## use sourcetrait_common::{cereal, datum, subsys}
Common comes through the workspace's `sourcetrait_common` and its facade, as
every crate takes it (User): cereal for the data, `datum`'s nonce and nom for a
request's identity and its host, and `subsys`'s error for a reply of the wrong
kind. `cereal::derived` emits absolute `::serde`, `::bitcode`, and `::rkyv`
paths, so those three are the crate's own dependencies.

## pub use
The capability set, which every side advertises; the engine's protocol, its
requests, commands, and answers with their diagnostics, error, and documents;
the value bounds and their checks; and nuval's data under its own names, the
value, its error and result, and every kind, so one value and error
representation serves every endpoint without a second hierarchy.
