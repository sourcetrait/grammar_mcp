# lib.rs

## use sourcetrait_common::cereal
The protocol's data is cereal data, so the crate takes `::serde`, `::bitcode`,
and `::rkyv` as dependencies of its own, the paths `cereal::derived` emits, as
the engine and the operator do.

The crate is the portable side of the daemon split: typed data alone, with no
evaluator, operator, MCP server, or platform monitoring in its dependencies.
The client, the daemon, the MCP frontend, and the probe all take it, and it
takes none of them, so the package graph stays one-way.

## pub use
The capability set, which every side advertises. The engine's requests and
answers, the runtime's, the values, and the framing join it as their own
modules.
