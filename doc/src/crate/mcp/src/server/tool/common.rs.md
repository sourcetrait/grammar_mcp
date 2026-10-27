# common.rs

## struct NuSh
One instance per process. The rmcp `#[tool_handler]` dispatches off `self.tool_router`,
which is assembled deny-filtered at construction.

### field system
Every eval, every equip action, and every signature block is the engine's answer to a
request that goes through mcp's system (`server/system.rs`), the same system that
observes the engine for the operator and shuts both down; the handle is shared with
the signal sweep.

### field interface_engine
Built once, from the deny set `NuSh::new` is handed, so a denied command is unknown to the
parse for the host's whole life; nothing evaluates on it, so it is shared rather than
cloned per call.

### field channel
The process-wide handle, NOT a fresh one. An eval reaches the channel through the same
global, so a per-NuSh handle would diverge from what bodies actually emit on.

### field remote_links
A clone of link.rs's process-wide registry, so the tools and the grimm sends see one
map. It is keyed by alias in both roles: a listener registers its link under the alias
it opened, as a connector does.

### field channel_open_lock
ASYNC because it is held across an await; the channel's own state lock is a std Mutex
precisely because the emit path must never need a runtime handle. The two locks differ in
kind for opposite reasons, which is worth not "simplifying".

## fn NuSh::new
The deny set is a PARAMETER rather than read from the config, because the one-shot CLI
builds its interface with none: deny gates an agent's surface, and the operator's path
keeps every command.

The `mcp_nom` is the caller's, minted before mcp's system starts, since the system's
operator and emergency log are keyed by it too.

## fn NuSh::mint
EVERY TOOL MINTS ITS NONCE FIRST, before any check, from its name and its parameters,
so every envelope, a refusal's included, carries one. The name rides in the hash beside
the parameters, so two tools given the same parameters at the same instant still part.
The request carries the nonce to the engine, which runs the call under it.

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
(`json_value_to_nu_value`), the crate's one JSON-to-Value path. `execute` and `rerun` hand
their checked value to `success_value` as it is, so a duration stays a duration. A void
tool answers `{nonce, result: null}`.

## fn answer_value
An eval's value crosses back as a `NuVal` and is rebuilt here into the nushell value the
envelope renders. A request the system could not answer and a refusal the engine made fold
into one error, which the tool's envelope carries either way.
