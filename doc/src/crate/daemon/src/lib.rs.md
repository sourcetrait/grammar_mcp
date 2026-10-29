# lib.rs

The persistent worker's runtime as a library, and grammard over it. The
library builds without the `service` feature, so an embedding host links the
runtime without the command line, the user service, or MCP; grammard is the
binary the `service` feature adds, its adapters over the same library.

## mod artifact, bridge, event, host, operation, peer, rpc, session, transport
Homes, empty until their code arrives, each a directory of submodules:
`artifact` the artifact store, `bridge` the controller's subsystem bridge,
`event` the event broker, `host` the engine's host services, `operation` the
operation journal, `peer` the retained mTLS remote Channel manager, `rpc` the
dispatch, `session` the controller's lease, and `transport` the listeners and
their admission.

## mod cli, mod run
grammard's command line and entry, behind `service`, the local CLI's declared
stub among them (`cli/oneshot.rs`).

## pub use
The runtime and its identity, the error and its kind, and, with `service`,
grammard's entry.
