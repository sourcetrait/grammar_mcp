# polars.rs

## fn main
Upstream's `main` at the engine's version, over its library, so the plugin
speaks the nu-plugin-protocol of the MCP that registers it.

`POLARS_ALLOW_EXTENSION` is set before the plugin builds anything, as
upstream sets it: polars' extensions make aggregates over object dtypes work,
and polars leaves them off by default for their unsafe code. The `unsafe`
block is `set_var`'s, sound here because no other thread exists yet.
