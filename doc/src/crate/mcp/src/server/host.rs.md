# host.rs

## fn init_engine
The engine's start: its parameters from the config, its homes the namespace's
(`home.rs`), and mcp its host. No engine can be built before it, so the test
driver and the one-shot CLI start it first, both under `embedded-cli` and never
on the serving path (`run.rs`), and the test harness once a process
(`guts.rs`).

The scratch directories are made here, before the engine, because parameters
carry them: every definition reads them as `$env.EQUIP_TMP` and
`$env.EQUIP_TRANSIENT` (`scratch.rs`). A directory that cannot be made fails the
start, as any other parameter would. The process's nom rides the parameters the
same way, a definition's `$env.EQUIP_MCP_NOM`.

## fn ensure_substrate
The engine's equip store first, then mcp's purview bookkeeping, the plugin
registry, and the builtins, each where the next needs it. A new namespace gets
the builtins and their purviews; an existing one is never touched, so a builtin
a user uninstalled stays uninstalled, and nothing installs again on their
behalf.

`ensure_default_purview` is non-fatal: purview bookkeeping must not stop the
host from serving.

`ensure_plugin_registry` is non-fatal for the same reason, and runs before the
builtins, since grammar's rig calls plugin commands its validation must know. It
belongs to the substrate because every engine base loads the registry it writes,
so it must land before the first base is built, on every entry point that
starts an engine: the test driver, the one-shot CLI, and the test server.

The engine starts after it, its locks hydrated from the store this makes, so
they hold the builtins installed a moment before (`run.rs`).

## struct McpHost
What an eval asks of mcp runs here, on the eval's thread, inside the `grimm`
command that asked: the Channel, the remote links, the config, and the pins are
mcp's, and the engine reaches them through this one value. The Channel and the
peer links it reaches through the bridge (`server/channel/bridge.rs`), blocking
on their runtime from the eval's thread, until HostRoute routes the engine's
asks through its controller.

### fn channel_send
The Channel's system takes the send whole, each step in its order there
(`crate/channel`'s `plan_send`): the reservation, the gate and its unverified
teardown, the spam count, the files and the spill, and the emit. What stays
here is the eval's side: the event made data, and on a stop verdict the
offender stopped through `stop`, its signals and its job, the action it reports
sent back for the error's emergency. With no face running there is no Channel
to send on, the same refusal a closed one gives.

### fn remote_send
The eval's side of a remote send: the reservation, the files read, and the
event rendered, each a failure of the sender's own, surfaced at the call. The
peer links' system takes the rest, the id, the spill, and the frames queued on
the link to `mcp_nom` (`crate/peer`'s `system.rs`), and holds the reservation
again on its side. With no face running there is no link to send on.

## fn sendable
A value nuval can't carry is refused with its render's error, the words the
send gave before the Channel took its event as data.

## fn read_attachments
Reads each {src, dest} row into its payload synchronously, before the id is
returned: a local src read failure is the agent's own problem and must surface
at the call, not as a later async Unsent. safe_dest guards every dest (relative,
no `..`) here too, so a bad dest fails the send outright rather than being
caught only on the receiver.

## fn config_record
The record `grimm get_config_all` returns mirrors the config file - the same
tables, the same key names - so one shape describes the file, the `.nutype`
model beside it, and what a definition sees. The remotes file stays out: a
definition addresses a linked host by its nom, which `interface remote channel
list` reports, never by the file's entry.

`identity` is null when the file names none, rather than absent, so the record's
shape holds whatever the file says.

The argument-owned values (`--id`, `--namespace`, `--workdir`) are deliberately
absent: they are not config-file settings, and a definition already reads them
ambiently as the `EQUIP_*` trio.

Values are live rather than as-launched. The channel's spam thresholds come from
the running Channel's system (`live_thresholds`), which `interface channel
config` updates, and the supervisor lines come through the pin layer - so what a
definition reads is what is actually in force rather than the startup seed. A
startup seed nobody is using would be a lie with a timestamp; with no Channel
running, the config's start is what is in force.

It reads through the operator's `effective_supervisor`, the config's lines its
base, rather than `config().supervisor` alone, which is what keeps a pin from
being something the watchdog honors while a read reports the old value.
