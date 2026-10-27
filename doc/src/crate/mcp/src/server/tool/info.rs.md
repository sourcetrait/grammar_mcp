# info.rs

`interface info` exists because the rmcp init handshake's `serverInfo` is not relayed into
agent context by Claude Code. The command surfaces the same facts through `tools/call` -
including the LIVE callable surface, so no stale skill index is needed.

## struct InfoResult

### field id
The agent's self-confirmation channel for which namespace it is talking to. The same pair
is readable ambiently inside a body as `$env.EQUIP_ID` / `$env.EQUIP_NAMESPACE`, so this
is the OUTSIDE view of the same fact.

### field mcp_nom
A CHANGED value across two calls is how an agent learns the server RESTARTED. A pid cannot
serve that, because pids are reused.

### field nu_version
Baked by `build.rs` from the workspace lockfile's `nu-protocol` pin, so it reports what is
actually LINKED rather than what a manifest asked for.

### field signatures
Read straight from each rig's committed index - no `.nu` walk and no parse - which is what
makes the agent's first read after the skill cheap.

### field purview
Last in the struct because it is the FRAME around the block rather than part of it.

## fn info
A purviews file that EXISTS but will not decode is an ERROR, never a silently empty and
therefore silently total view. The rig index already holds itself to that rule and this
follows it.

THE CURRENT VIEW ALONE: the command takes no argument, so what it renders is what the
session has in view, and `interface purview set` is how a view changes (User).

THE VIEW IS THE HOST'S, THE BLOCK THE ENGINE'S: purviews are mcp's own state, so the host
resolves the session's view into patterns and the engine's `Signatures` renders the rigs
within them.

FILTERING EXPANDS `@` references while the reported `purview` field carries the RAW
configuration - the two go through different functions on purpose, so a report shows what
was written and the filter matches what it means.
