# lib.rs

grammard-probe, the qualification driver: a test-only package that never
enters a product package. It drives the client and protocol APIs directly
for adversarial cases and, as its cases land, the MCP candidate over stdio.

## mod cases
The scenario groups' code, a file a group: `wire` so far.

## mod ser
serde's `Serialize` under one path: the inventory and the results are NUON,
written through nuval, and the manifests NUON its models hold, read through
it. The models are `.nutype` files: `type/` holds the candidate set's, the
inventory's, and the result's, and `tests/daemon/IdentityManifest.nutype` the
fixture's.

## pub use
The entry alone; the probe is its binary.
