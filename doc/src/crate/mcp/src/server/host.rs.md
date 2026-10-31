# host.rs

## fn init_engine
The engine's start: its parameters from the config and its homes the namespace's
(`home.rs`). No engine can be built before it, so the test driver and the one-shot
CLI start it first, both under `embedded-cli` and never on the serving path
(`run.rs`), and the test harness once a process (`guts.rs`).

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

## struct HostRoute
What an eval asks of mcp is answered here: the Channel, the remote links, the
config, and the pins are mcp's. An eval asks its engine's controller, mcp's
system, which sends each ask up to its own controller, this one
(`system.rs`'s `EngineWatch`), since a subsys system takes only data at its start
and so cannot hold the face's handles. It holds the face's two systems, the
Channel's and the peer links', and routes each ask to the one it is for. It
holds the emergency policy too (`emergency.rs`), its answer to its systems'
emergencies.

### fn of
The policy logs to the host's per-process emergency log and announces on the
face's Channel.

### fn supervise
Supervision's start, before mcp's system starts: the face's hub slot takes the
policy, so the Channel's spam pair is answered as the operator's emergencies
are. The one-shot CLI and the in-process test harness don't supervise: they
start no operator, and their Channel's spam pair goes unlogged and
unannounced.

### fn answer
A send goes to its system whole; the config is read here, with what each system
holds in force; a pin is the operator crate's live layer, which its watchdog
reads, whether or not an operator runs, since an unsupervised host pins and
reports its pins as a supervised one does.

### fn channel_send
The Channel's system takes the send whole, each step in its order there
(`crate/channel`'s `plan_send`): the reservation, the gate and its unverified
teardown, the spam count, the files and the spill, and the emit. A stop verdict
goes back to the sender, which stops its own eval and reports the action
(`HostAsk::Stopped`), and the report goes on to the Channel's system for the
error's emergency.

### fn remote_send
The sender's files are read and its event rendered off the runtime's workers, a
blocking read of the caller's own files; then the peer links' system mints the
id, spills an oversized event, and queues the frames on the link to `mcp_nom`
(`crate/peer`'s `system.rs`). The reservation was refused on the eval's side
first, as it always came before the files.

### fn on_packet
A packet mcp's system sends up unasked is an operator's emergency, which the
policy answers on the control task itself. The control task awaits each before
it takes the next, so the emergencies are logged and announced in the order the
operator raised them; the announcement is a request to the Channel's system,
never to mcp's, so it never waits on the control task that awaits it.

## fn prepared
The files are read before the event renders, the order the send's refusals
have always had.

## fn read_attachments
Reads each {src, dest} row into its payload, before the id is answered: a local
src read failure is the agent's own problem and must surface at the call, not as
a later async Unsent. safe_dest guards every dest (relative, no `..`) here too,
so a bad dest fails the send outright rather than being caught only on the
receiver.

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
the Channel's system (`live_thresholds`), which `interface channel config`
updates, and the supervisor lines come through the pin layer - so what a
definition reads is what is actually in force rather than the startup seed.

It reads through the operator's `effective_supervisor`, the config's lines its
base, rather than `config().supervisor` alone, which is what keeps a pin from
being something the watchdog honors while a read reports the old value.

## fn live_thresholds
A Channel's system that cannot answer leaves the config's start, the thresholds
the system began on.
