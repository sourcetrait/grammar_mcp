# lib.rs

The manifest carries no prose of its own by convention, so what follows is only
what a reader of it could reasonably get wrong.

## cfg_attr allow(dead_code, unused_imports)
A build without `embedded-cli` serves a narrow surface over the shared facade:
the Channel's send path, the emergency lines, and the interface catalog compile
whole, their breadth serving the test driver. The allow is that build's alone.
The `embedded-cli` build compiles every module with both lints, and clippy runs
over it, so code dead there still fails.

## cfg embedded-cli
The engine, the operator, nix, and every module that reaches them - the host,
the system, the operator's watch, the purviews, the one-shot, the test driver's
server, and `guts` - compile under `embedded-cli` alone, as does each tool's
engine-side method; each tool's parameters stay in every build, since the
interface parse carries them. The client, the Channel, the remote channels and
their links, the cert profile, the interface parse, the config, and the scratch
directories compile in every build: the Channel and the remote channels are the
MCP's own, never the runtime's (User).

## mod schema
The shim re-exporting `schemars::JsonSchema`, reached only through the
`schema::JsonSchema` path on a derive.

## mod json
serde_json's `Value` and `to_value` for `guts` alone, whose tool list is the
JSON-RPC's own `tools/list`; nothing the host serves converts through JSON.

## use sourcetrait_grammar_engine
The engine's public face named bare, as mcp's own items are, and the crate
itself as `engine`, the path its `init` is called by, under `embedded-cli`.

## use futures_util
`SinkExt` and `StreamExt`, and `PemObject` beside them, ride at CRATE ROOT
rather than inside a shim. They are TRAITS, so method dispatch needs them in
scope, and a shim would park them at `shim::Trait` where the calls in the
channel hub could not see them. That is the manifest's trait exception rather
than an oversight.

## mod guts
The one `pub mod` that is not part of the public face. The integration tests are
separate crates and therefore see only `pub` items, so the test-support harness
has to be public to be reachable at all; `guts` is the convention's name for
that, chosen to signal both its internal-only status and the bravery of anyone
outside the workspace reaching for it.

## pub use
The whole public surface: the CLI's parse, the deniable tools, the error, the
Channel's mode, which the error's Channel refusals carry, and the two entries,
`run_main` and `run_with`. Everything else is crate-internal.
