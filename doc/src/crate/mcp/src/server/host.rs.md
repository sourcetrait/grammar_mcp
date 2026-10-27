# host.rs

## fn init_engine
The engine's start: its parameters from the config, its homes the namespace's
(`home.rs`), and mcp its host. No engine can be built before it, so the server and
the one-shot CLI start it first (`run.rs`), and the test harness once a process
(`guts.rs`).

## fn ensure_substrate
The engine's equip store first, then mcp's purview bookkeeping, the plugin registry,
and the builtins, each where the next needs it. A NEW namespace gets the builtins and
their purviews; an existing one is never touched, so a builtin a user uninstalled stays
uninstalled, and nothing installs again on their behalf.

`ensure_default_purview` is non-fatal: purview bookkeeping must not stop the host from
serving.

`ensure_plugin_registry` is non-fatal for the same reason, and runs BEFORE the builtins,
since grammar's rig calls plugin commands its validation must know. It belongs to the
substrate because every engine base loads the registry it writes, so it must land before
the first base is built, on every entry point: the server, the one-shot CLI, and the test
server.

The engine starts AFTER it, its locks hydrated from the store this makes, so they hold the
builtins installed a moment before (`run.rs`).

## struct McpHost
What an eval asks of mcp runs here, on the eval's thread, inside the `grimm`
command that asked: the Channel, the remote links, the config, and the pins are
mcp's, and the engine reaches them through this one value.

### fn channel_send
ORDER IS DELIBERATE THROUGHOUT, and each step is placed where it is for a reason.

The `mcp/` REFUSAL comes first, before the channel state is even consulted. A
definition is not the host, so letting it stamp a model under that prefix would let
it forge a host control packet - and would destroy the property the reservation
exists for, that provenance is checkable from the path alone. This is the one place
a non-host picks a model, which is what makes it the right place to enforce it.

The PHASE is checked before any rendering, so a send that cannot happen costs a
lock and an error rather than a hash and a render.

OPEN-BUT-UNVERIFIED does not merely refuse - it TEARS THE CHANNEL DOWN. Telemetry
must not reach a peer that has not proven it owns the stdio session, and a peer
that has been emitted to unproven is not a peer we keep.

The phase is read HERE, at send time, never captured: a job outliving its eval
keeps its decl and everything it closed over, so a spawn-time snapshot would let
work started before verification emit to an unproven peer.

On the hard threshold the engine stops the offender through `stop`, its signals and
its job, and the action it reports rides the emergency.

The id is minted BEFORE the attachment write because the filename derives from it,
which is also why the hash covers the attached CONTENT rather than its path.

After the id and the caller's attachment, the rendered event is checked against
the notification cap (event_overflows, state.rs); an oversized event spills to
inbox/<id>.event.nuon and a compact pointer rides the wire in its place. The
pointer names its own spill file, so it is orthogonal to the caller's `attached` -
a big event and a caller attachment both survive, in separate files.

### fn remote_send
The `mcp/` reservation first, as for a Channel send; then the attachments read, the
event rendered, the id minted, any spill made, and the send enqueued on the open
link to `mcp_nom` (`find_link_send`, link.rs).

## fn write_inbox_file
The one writer for both of a send's files: the caller's attachment, `<id>.nuon`,
and an auto-spilled event, `<id>.event.nuon`.

## fn read_attachments
Reads each {src, dest} row into (dest, bytes) SYNCHRONOUSLY, before the id is
returned: a local src read failure is the agent's own problem and must surface at the
CALL, not as a later async Unsent. safe_dest guards every dest (relative, no `..`)
here too, so a bad dest fails the send outright rather than being caught only on the
receiver.

## fn spill_remote_event
The sender's half of the event spill: an event that would overflow the receiver's
notification cap (event_overflows, state.rs) moves to a transferred file - the
reserved EVENT_SPILL_DEST, pushed beside any caller files - and a compact pointer
replaces it on the wire, for both decls. It is the sender's act, through the file
connection, rather than the receiver's render choice, because an event over 1 MiB
cannot ride the inline Deliver frame at all, so only the file transfer reaches the
agent. The pointer's path is dest-relative; the receiver rewrites it under the
delivery's inbox directory (link.rs's rewrite_spill_pointer), so the agent resolves it
as <inbox>/<spilled_event_path>, exactly as for a local channel send's spill.

## fn config_record
The record `grimm get_config_all` returns mirrors the TOML - the same two tables,
the same key names - so ONE shape describes the file, the `.nutype` model beside it,
and what a body sees.

The argument-owned values (`--id`, `--namespace`, `--workdir`) are deliberately
absent: they are not config-file settings, and a body already reads them ambiently
as the `EQUIP_*` trio.

VALUES ARE LIVE RATHER THAN AS-LAUNCHED. The channel's spam thresholds come from
the channel handle, which `interface channel config` mutates, and the supervisor
lines come through the pin layer - so what a body reads is what is actually in force
rather than the startup seed. A startup seed nobody is using would be a lie with a
timestamp.

It reads through `effective_supervisor()` rather than `config().supervisor`, which
is what keeps a pin from being something the watchdog honors while a read reports
the old value.
