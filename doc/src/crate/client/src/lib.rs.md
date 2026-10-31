# lib.rs

The MCP's side of a session with a remote worker. It takes the protocol and
portable utilities alone, never the engine, the operator, or the daemon, so a
product that drives a worker links it without an engine of its own. The SSH
transport that opens a session joins it with RemoteClient.

## mod session
The MCP's end of a session over any reader and writer, an SSH channel's or a
child's pipes, so the daemon's tests drive a real worker through the same
code the MCP will.

## pub use
The session, its events, and an ask's responder.
