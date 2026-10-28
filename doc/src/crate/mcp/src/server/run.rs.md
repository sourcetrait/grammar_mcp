# run.rs

## fn run_server
The order here is load-bearing, top to bottom.

`install_child_subreaper` comes first so an eval's orphaned grandchildren stay on
our /proc ppid chain for the tree-kill. Everything after it can spawn externals.

Mcp's system starts after the substrate: its engine hydrates the equips' locks
from the store the substrate makes, builtins included, so an engine started first
would answer a call to a builtin `equip::not_registered`. It brings up the
emergency lane, the operator, which takes the namespace's host lock, and the
observer with the engine (`server/system.rs`). The `mcp_nom` is the process's,
drawn for the engine's start (`process_identity`, `server/host.rs`), and the
system's operator and emergency log and `NuSh` all carry it.

The signal sweep is installed before serving. Without it a signalled host drops
its eval children and its background jobs onto the box unreaped, because the
stdio path only ever learns about a clean client disconnect. None of it reaches
SIGKILL - that is what the host lock is for, and the next host's sweep of the
scratch directories (`scratch.rs`).

Nothing from outside panics it. The substrate, mcp's system and its components,
the stdio handshake, and the service's join each come back as an `Err` naming
what failed, which `serve_or_oneshot` prints and exits 1 on. rmcp's `serve`
refuses a first message it cannot take, a notification before any request, and
that is the client's error to report, never a host panic. Claude Code's
`server/discover` probe is no such message: rmcp answers it and stays open for
either protocol era.

The shutdown path runs once the service ends, whatever ended it: the channel is
told we are going away, then mcp's system stops its components in order, every
in-flight eval cancelled and reaped first, so a disconnect mid-eval leaks no
process tree.

## const CLOSE_FLUSH_GRACE
Bounded because a peer that has already gone must never hold the shutdown open.

## fn close_channel_for_shutdown
Without this a shutting-down host simply vanishes and the agent sees a bare
1006, indistinguishable from a crashed host - the very distinction the explicit
close frame exists to provide.

The frame is written by the hub task, so the wait is an ack rather than a sleep:
the hub fires the completion after its flush. A sleep long enough to usually
work would be a race dressed as a guarantee.

## fn spawn_signal_sweep
Exiting is part of the contract: handling a termination signal without
terminating would make the host ignore the very thing it was told to do.

It is also what makes the handler observable. A swept host reports
`code() == Some(128 + signo)`, while a host that merely took the signal's
default action reports `code() == None` and `signal() == Some(signo)`. Only the
former proves the sweep ran, which is what the system test measures.

The scratch directories go last, after the components that write into them have
stopped: `process::exit` never returns to `serve_or_oneshot`, whose ordinary end
would remove them otherwise.
