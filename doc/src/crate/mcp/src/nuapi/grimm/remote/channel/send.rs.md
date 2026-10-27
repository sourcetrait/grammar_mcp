# send.rs

Grimm decls a definition calls, registered by `register_nuapi`, NOT MCP tools - an
agent's willful, non-blocking send to a linked peer. Both mint a message id off the
channel's NonceGenerator, look up the open link by mcp_nom (find_link_send,
link.rs), enqueue, and return it IMMEDIATELY as `{event_id}`; the delivery outcome
lands async as mcp/remote/Sent or Unsent on the sender's own Channel, driven by the
transport WRITE (link.rs's SendTracker - Sent covers the whole transmission, files
and message). The aggregate-write semantics and the receiver's relay live in
link.rs; these decls are the thin surface over them.

## struct GrimmRemoteChannelSend / struct GrimmRemoteChannelSendWith
Two decls, not one with an optional attachment, because for a REMOTE send the
transferred FILES are the "attached" - _send_with's whole job - so a bare send has no
files at all (unlike channel send's optional `attached`). The model refuses the mcp/
reservation, host-origin only, as channel send's does.

### fn read_attachments
Reads each {src, dest} row into (dest, bytes) SYNCHRONOUSLY, before the id is
returned: a local src read failure is the agent's own problem and must surface at the
CALL, not as a later async Unsent. safe_dest guards every dest (relative, no `..`)
here too, so a bad dest fails the send outright rather than being caught only on the
receiver.

### fn spill_remote_event
The sender's half of the event spill: an event that would overflow the receiver's
notification cap (event_overflows, state.rs) moves to a transferred file - the
reserved EVENT_SPILL_DEST, pushed beside any caller files - and a compact pointer
replaces it on the wire, for both decls. It is the sender's act, through the file
connection, rather than the receiver's render choice, because an event over 1 MiB
cannot ride the inline Deliver frame at all, so only the file transfer reaches the
agent. The pointer's path is dest-relative; the receiver rewrites it under the
delivery's inbox directory (link.rs's rewrite_spill_pointer), so the agent resolves it
as <inbox>/<spilled_event_path>, exactly as for a local channel send's spill.
