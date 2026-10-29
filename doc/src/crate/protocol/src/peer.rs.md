# peer.rs

The remote Channel's portable data: its models, its rows, and its events, in
the shapes the agent already reads. The links are the MCP's own, apart from the
remote engine: a remote Channel carries events, messages, and files between
hosts, never an engine request, so nothing here names one.

## const MODEL_CONNECTED / MODEL_DISCONNECTED / MODEL_SENT / MODEL_UNSENT
The host's lifecycle and delivery models, under the `remote/` reservation, which
no definition or peer may claim.

## struct RemoteChannelRow
One row an alias, the file's entry beside its live state, so the agent reads
what it configured and what is running in one place: `endpoint` is the bound or
dialed address while live, and `remote_mcp_nom` the peer's nom once paired,
which is what Sent, Unsent, and a relayed packet's `from` carry. Its `state`
serializes as the word the row has always carried, `closed`, `listening`, or
`open`.

## enum RemoteChannelEvent
Each packet's event as data. `Disconnected` carries `mcp_nom` when a link was
established and `error` when it failed, neither for a listener closed before it
paired, so a consumer keys a link's lifecycle on `remote` and reads the rest by
presence. `Sent` reports the write to the link, never that an agent processed the
message.
