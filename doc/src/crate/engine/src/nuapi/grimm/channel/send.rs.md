# send.rs

## struct GrimmChannelSend
THE CHANNEL IS A NOTIFICATION LANE, NOT A SERIALIZATION LANE. `event` says "state
changed, here is a tiny summary"; the agent fetches the actual data itself. That
premise is why the signature has TWO slots rather than one: an author with
something bulky puts it in `attached`, which the host writes to the inbox and
NAMES on the wire rather than carrying. The contract is STRUCTURAL - the ergonomic
path is the next argument - so nobody has to be talked out of putting data on the
wire.

It returns the message id as `{event_id}`, the record its signature declares, so
an author can correlate what it sent with what the agent later fetches.

### fn run
The arguments are checked here, the event and any attachment a record or a table,
and the sending call's origin read from `$env.NONCE`. Everything after is the
host's (`EngineHost::channel_send`): the `mcp/` refusal, the phase, the spam gate,
the id, the inbox, the spill, and the emit, in the order mcp's `server/host.rs`
gives its reasons for. The engine keeps the one act on its own state, stopping
the offender, which the host calls back for through `stop`.

## fn shell_error
`GenericError` renders its TITLE through Display, and the title is all the eval
envelope's `message` carries - so a bare command name there reaches the agent with
the cause stripped off. That is the opaque-error failure this crate already knows
well, and it recurs because the label looks like the natural home for detail.

## fn stop_offender
The host calls it through `stop` when a send crosses the hard threshold, and the
action it returns rides the emergency.

TWO LEVERS, because the offender has two shapes and neither lever reaches both.

`Signals::trigger` reaches a spam loop whatever it runs under: such a loop invokes
a decl over and over, so it polls at every one of those boundaries and bails.

But a job that OUTLIVED the eval that spawned it has no `in_flight` entry
left - `InFlightCleanup` removed that the moment the dispatch returned - and the
jobs table is the only place it is still reachable. A spawned closure reads its OWN
JobId here, so `current_job` names the actual offender rather than its parent.

PROVEN LIVE: a detached `job spawn` whose loop CAUGHT every throw and had 196
iterations left still stopped at the fifth send, and the jobs table went empty.
That is the case the throw alone cannot cover, and the table emptying is the half
`Signals` could never do. The `action` string records which branch ran, which is
also how it is verified.

Only the PROCESSES of a killed job can fail to die; the table entry goes either
way, and an external the job left behind is the tree-kill machinery's to reap.
