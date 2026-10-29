# lib.rs

The crate is the portable side of the daemon split: typed data alone, with no
evaluator, operator, MCP server, or platform monitoring in its dependencies.
The client, the daemon, the MCP frontend, and the probe all take it, and it
takes none of them, so the package graph stays one-way.

## mod cereal
cereal's derive macro taken directly, under the one path its output names,
`cereal::`: the workspace's `sourcetrait_common` umbrella carries its whole
feature set, clap and the test helpers among them, into every crate that takes
it, and workspace inheritance can add features but never remove them. The
protocol's own types need the macro alone. `cereal::derived` emits absolute
`::serde`, `::bitcode`, and `::rkyv` paths, so those three are the crate's own
dependencies.

## use nuval::wide
The wire's widening, shared with nuval so every count and offset widens one
way.

## pub use
The capability set, which every side advertises; the value bounds and their
checks; and nuval's data under its own names, the value, its error and
result, and every kind, so one value and error representation serves every
endpoint without a second hierarchy.
