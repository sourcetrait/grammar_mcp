# send.rs

Grimm decls a definition calls, registered by `register_nuapi`, NOT MCP tools - an
agent's willful, non-blocking send to a linked peer. Both hand the send to the host
(`EngineHost::remote_send`), which mints a message id off the channel's
NonceGenerator, looks up the open link by mcp_nom, enqueues, and returns it
IMMEDIATELY as `{event_id}`; the delivery outcome lands async as mcp/remote/Sent or
Unsent on the sender's own Channel, driven by the transport WRITE (mcp's link.rs,
its SendTracker - Sent covers the whole transmission, files and message). The
aggregate-write semantics and the receiver's relay live in link.rs; these decls are
the thin surface over them.

## struct GrimmRemoteChannelSend / struct GrimmRemoteChannelSendWith
Two decls, not one with an optional attachment, because for a REMOTE send the
transferred FILES are the "attached" - _send_with's whole job - so a bare send has no
files at all (unlike channel send's optional `attached`). The host refuses the mcp/
reservation, host-origin only, as it does for a channel send.
