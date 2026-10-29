# host.rs

## fn init_engine
The engine's start: its parameters from the config, its homes the namespace's
(`home.rs`), and mcp its host. No engine can be built before it, so the server
and the one-shot CLI start it first (`run.rs`), and the test harness once a
process (`guts.rs`).

The scratch directories are made here, before the engine, because parameters
carry them: every definition reads them as `$env.EQUIP_TMP` and
`$env.EQUIP_TRANSIENT` (`scratch.rs`). A directory that cannot be made fails the
start, as any other parameter would. The process's nom rides the parameters the
same way, a definition's `$env.EQUIP_MCP_NOM`.

## fn process_identity
One nom a process, drawn at the first ask. The engine's start needs it before
the server or the one-shot builds its system, so both take this one rather than
minting their own, as does every `TestServer` in a test binary, whose engine is
the process's too. The generator rides with it: the nonces a host mints come
from the generator that drew its nom, as they did when the server minted both.

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
so it must land before the first base is built, on every entry point: the
server, the one-shot CLI, and the test server.

The engine starts after it, its locks hydrated from the store this makes, so
they hold the builtins installed a moment before (`run.rs`).

## struct McpHost
What an eval asks of mcp runs here, on the eval's thread, inside the `grimm`
command that asked: the Channel, the remote links, the config, and the pins are
mcp's, and the engine reaches them through this one value.

### fn channel_send
Order is deliberate throughout, and each step is placed where it is for a
reason.

The `mcp/` and `remote/` refusal comes first, before the channel state is even
consulted. A definition is not the host, so letting it stamp a model under
either prefix would let it forge a host packet - and would destroy the property
the reservation exists for, that provenance is checkable from the path alone.
This is where a definition picks a model, which is what makes it the right
place to enforce it; a peer's relay is held to the same check (`link.rs`).

The phase is checked before any rendering, so a send that cannot happen costs a
lock and an error rather than a hash and a render.

Open-but-unverified does not merely refuse - it tears the channel down.
Telemetry must not reach a peer that has not proven it owns the stdio session,
and a peer that has been emitted to unproven is not a peer we keep.

The phase is read here, at send time, never captured: a job outliving its eval
keeps its decl and everything it closed over, so a spawn-time snapshot would let
work started before verification emit to an unproven peer.

On the hard threshold the engine stops the offender through `stop`, its signals
and its job, and the action it reports rides the emergency.

The id is minted before the attachment write because the filename derives from
it, which is also why the hash covers the attached content rather than its path.

After the id and the caller's attachment, the rendered event is checked against
the notification cap (event_overflows, state.rs); an oversized event spills to
<inbox>/<id>.event.nuon and a compact pointer rides the wire in its place. The
pointer names its own spill file, so it is orthogonal to the caller's
`attached`: a big event and a caller attachment both survive, in separate files.

### fn remote_send
The reservations first, as for a Channel send; then the attachments read,
the event rendered, the id minted, any spill made, and the send enqueued on the
open link to `mcp_nom` (`find_link_send`, link.rs).

## fn write_inbox_file
The one writer for both of a send's files: the caller's attachment, `<id>.nuon`,
and an auto-spilled event, `<id>.event.nuon`.

## fn read_attachments
Reads each {src, dest} row into (dest, bytes) synchronously, before the id is
returned: a local src read failure is the agent's own problem and must surface
at the call, not as a later async Unsent. safe_dest guards every dest (relative,
no `..`) here too, so a bad dest fails the send outright rather than being
caught only on the receiver.

## fn spill_remote_event
The sender's half of the event spill: an event that would overflow the
receiver's notification cap (event_overflows, state.rs) moves to a transferred
file - the reserved EVENT_SPILL_DEST, pushed beside any caller files - and a
compact pointer replaces it on the wire, for both decls. It is the sender's act,
through the file connection, rather than the receiver's render choice, because
an event over 1 MiB cannot ride the inline Deliver frame at all, so only the
file transfer reaches the agent. The pointer's path is dest-relative; the
receiver rewrites it under the delivery's inbox directory (link.rs's
rewrite_spill_pointer), so the agent resolves it as
<inbox>/<spilled_event_path>, exactly as for a local channel send's spill.

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
the channel handle, which `interface channel config` mutates, and the supervisor
lines come through the pin layer - so what a definition reads is what is
actually in force rather than the startup seed. A startup seed nobody is using
would be a lie with a timestamp.

It reads through the operator's `effective_supervisor`, the config's lines its
base, rather than `config().supervisor` alone, which is what keeps a pin from
being something the watchdog honors while a read reports the old value.
