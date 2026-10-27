# common.rs

## fn write_definition
PRE-DISPATCH and for EVERY `nu` and `rerun` eval - success, error, or timeout alike. That
is what leaves a timed-out call rerunnable: its error envelope carries the nonce, and
`rerun(nonce, data, timeout: bigger)` recovers it.

Non-fatal by design: a write failure logs and the call proceeds, so the call still returns
its result and nonce and only a later rerun would miss the definition.

## enum InFlightKind
Every kind but `call` carries its LANE, the one thing `interface process kill` and the hang
watch need to tell a stuck stateful lane from a held stateless permit; `call` always runs
stateless.

## struct NuSh
One instance per process. The rmcp `#[tool_handler]` dispatches off `self.tool_router`,
which is assembled deny-filtered at construction.

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

## struct InFlightEntry
`interface process list` snapshots this and `interface process kill` reads it. The
visibility caveat is unchanged and deliberate: the list shows ALL in-flight work on the
host, so subagents see each other's calls and the primary's.

## fn NuSh::new
`nu::CRYPTO_PROVIDER.default()` installs nushell's TLS provider once so the `http` family
works. Nushell reads its OWN OnceLock here, never rustls's process-global, which is why
this call rather than a rustls one.

The deny set is a PARAMETER rather than read from the config, because the one-shot CLI
builds its interface with none: deny gates an agent's surface, and the operator's path
keeps every command.

## fn NuSh::mint
EVERY TOOL MINTS ITS NONCE FIRST, before any check, from its name and its parameters,
so every envelope, a refusal's included, carries one. The name rides in the hash beside
the parameters, so two tools given the same parameters at the same instant still part.

## fn envelope_text
ONE SEAM for every envelope, a success's here and a failure's through
`error_to_call_result` (error.rs): a nu record, `nonce` first, rendered by `render_nuon`
as the result's one text content. There is no structured content and no output schema;
the text is the result, NUON the agent reads as it reads its own data.

## fn success
A tool's own result struct crosses to nu through the serde bridge
(`json_value_to_nu_value`), the crate's one JSON-to-Value path. `nu` and `rerun` hand
their checked value to `success_value` as it is, so a duration stays a duration. A void
tool answers `{nonce, result: null}`.

## fn dispatch_pooled
THE HANDLER MINTS THE NONCE FIRST, and the dispatch hands it to the eval, which sets it as
`$env.NONCE` on the eval's stack - which is what makes the ambient value equal the
envelope's.

`refresh_base_if_stale` runs before taking a clone, so this eval runs against current
plugin decls rather than whatever the base held when it was built.

ONE DEADLINE covers the permit's wait and the eval: the call's timeout is what the agent
is willing to wait for the call, so the eval gets what the wait left. Without the bound,
a call arriving while every permit is held - hung threads holding them among the
causes - waited without end, outside `interface process list` and past its own timeout.
A lapsed wait is `thread::dispatch` naming the permits held against the cap, with the
nonce, since the call's log directory already exists.

The tracker is set as the engine's `background_thread_job` BEFORE the eval starts, because
nushell registers an external's pid at spawn time and a tracker attached later would miss
it.

ON TIMEOUT the order matters: trigger cancel so the abandoned thread bails at nushell's
next check point and releases its permit, THEN tree-kill the external process tree, THEN
register for hang watching. A pure-Rust hung eval cannot be reached by the first step -
that is the accepted residual - and the registration is what turns it into a reported
condition rather than a silent leak.

## fn dispatch_stateful
The stateful lane, a `def --env` definition's. The engine handle is cloned OUT of the guard
rather than held across the eval, because the engine thread serializes calls itself -
holding the guard would serialize them twice and block an unrelated caller for the
duration. Its timeout covers the wait for the lane as well as the eval.

## struct InFlightCleanup
Drop SPAWNS an async removal task because `Drop` cannot await, and the map is behind an
async mutex. The consequence worth knowing: removal is not synchronous with the dispatch
returning, so an `interface process list` racing a completion can still see the finished
entry briefly.

## fn teardown_all_in_flight
Called after the MCP service stops - the client closed stdin - and from the signal
handlers, so a disconnect or a signalled death mid-eval never leaks a process tree.

## fn sweep_env_jobs
THE HALF `in_flight` STRUCTURALLY CANNOT COVER. A `job spawn` that outlives its eval has
had its registry entry removed by `InFlightCleanup` the moment that dispatch returned,
while `env_jobs` still holds it - and its external child is exactly the one that would
otherwise keep running as the box user with its supervising thread already gone.

The pids are collected BEFORE `kill_all`, for two reasons: that call clears the table and
takes the tracked pid sets with it, and nushell's own kill reaches only the DIRECT child,
so the /proc descendant walk in `tree_kill` is what gets a grandchild.
