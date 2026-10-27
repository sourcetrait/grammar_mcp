# teardown.rs

nushell 0.116.1 runs each external of an `is_mcp` engine in its own session, `setsid`
before exec, so the external leads a process group of its own (`engine.rs`). A `ThreadJob`
tracks only that direct child's pid, and a descendant that starts its own session or group
leaves the external's, so neither the tracked pid nor a killpg reaches the whole tree: a
plain SIGKILL of the child orphans its grandchild.

The reap that works: a /proc descendant walk from each tracked child BEFORE the kill, which
makes it deterministic, plus SIGKILL on each, with the host the eval's subreaper so an
orphan reparents to it and stays on the /proc ppid chain (mcp's `server/teardown.rs`).
`Jobs::kill_and_remove` alone reaches only the tracked direct child.

All of it is Linux-gated. A non-target build degrades to a direct-child kill with no
/proc walk, which keeps the shipped crate honest by construction rather than by
assumption - we target Linux, and saying so in `cfg` is more honest than pretending
otherwise.

## fn make_tracker
Setting the returned job as an engine's `current_job.background_thread_job` is what makes
nushell register the pid of every external that eval spawns into an Arc-shared set, which
the kill and timeout paths then read.

The Mail sender is a THROWAWAY on purpose: pid tracking is sender-independent, and a
foreground eval never emits job Mail.

Its `Signals` share the eval's cancel flag, so once cancel fires no further pid is
registered.

## fn kill_plugin_subprocesses
Unblocks a plugin-hung eval, which nothing else can: a hung plugin ignores the cancel
`Signals` and is not a tracked external, so killing its subprocess is what closes the
plugin IPC pipe and makes the eval's plugin read return an error. Plugins respawn lazily
on next use.

BROAD BY NATURE - nushell shares plugin subprocesses across evals - so it fires only on
an explicit `Kill` and on the teardown, NEVER on an automatic timeout, where the
supervisor scopes that escalation instead.

## fn descendants_of
Walked BEFORE the kill, which is what makes the set deterministic rather than a race
against reparenting.
