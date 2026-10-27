# lib.rs

## mod nu
nushell's items the engine names, under one path, so a reader tells them from
the engine's own.

## use sourcetrait_common::cereal
`EngineError` is cereal data, so the engine takes `::serde`, `::bitcode`, and
`::rkyv` as dependencies of its own, the paths `cereal::derived` emits, as
nuval does.

## mod json
`JsonObject` is serde_json's map, the type rmcp's `JsonObject` names, so a
tool's arguments reach the templates and the schema converters as they arrive
and the engine takes no rmcp.

## pub use
What mcp names: the engine's start, its parameters, and its host; the two lanes,
the stateless pool's executor and the stateful engine; the definition's
contract, the lint, and the parse engines; the type grammar's converters and
check; the source templates; the plugin registry's read; the call cache's paths;
and the diagnostics and the error. Everything else is the crate's own.
