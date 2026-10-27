# lib.rs

The manifest carries no prose of its own by convention, so what follows is only
what a reader of it could reasonably get wrong.

## mod schema
The shim re-exporting `schemars::JsonSchema`, reached only through the
`schema::JsonSchema` path on a derive.

## use sourcetrait_grammar_engine
The engine's public face named bare, as mcp's own items are, and the crate
itself as `engine`, the path its `init` is called by.

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
