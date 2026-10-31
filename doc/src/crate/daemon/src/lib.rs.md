# lib.rs

The persistent worker's runtime as a library, and grammard over it. The
library builds without the `service` feature, so an embedding host links the
runtime without the command line, the user service, or MCP; grammard is the
binary the `service` feature adds, its adapters over the same library.

## mod artifact, bridge, event, host, peer, rpc
Homes, empty until their code arrives, each a directory of submodules:
`artifact` the artifact store, `bridge` the controller's subsystem bridge,
`event` the event broker, `host` the engine's host services, `peer` the
retained mTLS remote Channel manager, and `rpc` the dispatch. The operation
journal is a crate of its own, `crate/journal`, so another product takes it.

## mod controller
The worker's systems under the controller its process owns.

## mod session
The controller's lease and every open stream bound to it, each served by its
role.

## mod tests
The crate's own tests, against what an integration test can't reach: the
sessions on an injected clock.

## mod instance
The instance a worker starts as: its names, its key, its homes, and its file,
resolved before anything runs.

## mod runtime, mod transport
The worker's start, its lock, and its scratch; the engine socket's listener
and each stream's admission.

## mod cli, mod run
grammard's command line and entry, behind `service`, the local CLI's declared
stub among them (`cli/oneshot.rs`).

## pub use
The runtime and its identity, the worker's scratch, the instance's types, the
error and its kind, and, with `service`, grammard's entry.
