# lib.rs

The crate is the portable side of the daemon split: typed data alone, with no
evaluator, operator, MCP server, or platform monitoring in its dependencies.
The client, the daemon, the MCP frontend, and the probe all take it, and it
takes none of them, so the package graph stays one-way.

## use nuval::wide
The wire's widening, shared with nuval so every count and offset widens one
way.

## use sourcetrait_common::cereal
cereal comes through the workspace's `sourcetrait_common` and its facade, as
every crate takes Common (User). `cereal::derived` emits absolute `::serde`,
`::bitcode`, and `::rkyv` paths, so those three are the crate's own
dependencies.

## pub use
The capability set, which every side advertises; the value bounds and their
checks; and nuval's data under its own names, the value, its error and
result, and every kind, so one value and error representation serves every
endpoint without a second hierarchy.
