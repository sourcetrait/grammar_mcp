# common.rs

## struct NuSh
The test driver's server, under `embedded-cli` alone; the serving MCP is the client
(`server/client.rs`). One instance per process. The rmcp `#[tool_handler]` dispatches off
`self.tool_router`, which is assembled deny-filtered at construction.

### field system
Every eval, every equip action, and every signature block is the engine's answer to a
request that goes through mcp's system (`server/system.rs`), the same system that
observes the engine for the operator and shuts both down; the handle is shared with
the signal sweep.

### field interface_engine
Built once, from the deny set `NuSh::new` is handed, so a denied command is unknown to the
parse for the host's whole life; nothing evaluates on it, so it is shared rather than
cloned per call.

### field face
The Channel's commands and the remote channels', on the process's one Channel and its
links, the face the client serves them through too (`server/channel/face.rs`).

## fn NuSh::new
The deny set is a PARAMETER rather than read from the config, because the one-shot CLI
builds its interface with none: deny gates an agent's surface, and the operator's path
keeps every command.

The `mcp_nom` is the caller's, the process's one (`process_identity`), since the
system's operator and emergency log are keyed by it too.

## fn NuSh::mint
`mint_nonce` on the server's generator (`server/identity.rs`), the client's minting too.
The request carries the nonce to the engine, which runs the call under it.

## fn resolve_host
Every tool takes `host`, required, User's ruling: an agent working beside remote hosts,
possibly several, names where each call runs rather than leaving it to a default it might
assume wrongly. Each tool resolves it right after minting its nonce, so an unknown alias
is refused, nonce and all, before the tool's own checks. The match is exact, so a case
or a space makes another alias.

## fn unexpected
An answer of another command's kind is the engine breaking its own protocol, so it is
`internal`, never the agent's error.

## fn envelope_text
ONE SEAM for every envelope, a success's here and a failure's through
`error_to_call_result` (error.rs): a nu record, `nonce` first, rendered by `render_nuon`
as the result's one text content. There is no structured content and no output schema;
the text is the result, NUON the agent reads as it reads its own data.

## fn success
A tool's own result struct crosses to nu through the serde bridge
(`json_value_to_nu_value`, nuval's), the workspace's one JSON-to-Value path. `execute`, `rerun`, and
`call` hand their checked value to `success_value` as it is, so a duration stays a
duration. A tool answering nothing carries `{nonce, result: null}`.

## fn answer_value
An eval's value crosses back as a `NuVal` and is rebuilt here into the nushell value the
envelope renders. A request the system could not answer and a refusal the engine made fold
into one error, which the tool's envelope carries either way.
