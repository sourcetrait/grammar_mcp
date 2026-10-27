# remote_channels.rs

## fn remote_list
Two read-only sets from the process-global registries (remote_links and
bound_listeners, link.rs): `channels`, the established links, each {alias,
remote_mcp_nom, address}, and `listening`, the bound-but-unpaired listeners, each
{remote, address}, both sorted by alias. `channels` is where the agent correlates an
alias (what it opened) to the peer's mcp_nom (what Sent, Unsent, and a relayed packet's
`from` carry), since a clean Connected carries only the nom. `listening` makes a bound
listener visible before it pairs - it has no remote_mcp_nom, there being no peer yet -
and a listener that pairs moves from `listening` into `channels`.
