# remote_channels.rs

## struct RemoteRow
One row an alias, the file's entry beside its live state, so the agent reads
what it configured and what is running in one place: `endpoint` is the bound or
dialed address while live, and `remote_mcp_nom` the peer's nom once paired,
which is what Sent, Unsent, and a relayed packet's `from` carry.

## fn remote_rows
The file's entries and every live alias, an alias live but since unset among
them, so a link that outlives its entry stays visible.

## fn live_remotes
Each registry is read under its own lock, released before the next is taken:
`promote_listener` (`link.rs`) takes the listeners' lock and then the links',
so holding both here in the other order could deadlock.

## fn remote_to_open
A listen or a connect on a live alias is refused `remote::channel::already_open`, a
listener still waiting among them: a second listen would otherwise bind a
second port and leave the first's accept running.

## fn link_keys
The identity is the config's, read once at the start; the peer's certificate is
the entry's, expanded at each open, so an edited entry takes effect at the next
listen or connect.
