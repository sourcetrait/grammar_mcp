# lib.rs

The guard over owned scratch directories, shared by every process that keeps
them: the MCP's frontend, the worker, and the client. It holds neither the
engine nor the operator, so a client links it without either.

## mod dir, liveness, owner, sweep
The owned directory, its owner's liveness, the marker, and the sweep.
