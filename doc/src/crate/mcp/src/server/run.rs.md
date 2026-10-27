# run.rs

## fn run_server
THE ORDER HERE IS LOAD-BEARING, top to bottom.

`install_child_subreaper` comes first so an eval's orphaned grandchildren stay on our
/proc ppid chain for the tree-kill. Everything after it can spawn externals.

The EMERGENCY LANE is built before the watchdog, because the watchdog is its producer
and there is nothing to push onto until it exists. The channel's spam classifier is
installed onto the same lane here rather than at `NuSh` construction for the same
reason - the lane does not exist that early, which is also why the one-shot CLI's
channel simply has no responder.

The HOST LOCK is non-fatal on failure: two hosts may legitimately share one namespace,
and "locked" still answers the only question a watcher asks. A failure is logged and
the host carries on.

The SIGNAL SWEEP is installed before serving. Without it a signalled host drops its
eval children and its background jobs onto the box unreaped, because the stdio path
only ever learns about a CLEAN client disconnect. None of it reaches SIGKILL - that is
what the host lock is for.

NOTHING FROM OUTSIDE PANICS IT. The substrate, the stdio handshake, and the service's
join each come back as an `Err` naming what failed, which `serve_or_oneshot` prints and
exits 1 on. rmcp's `serve` refuses a first message it cannot take, a notification
before any request, and that is the client's error to report, never a host panic.
Claude Code's `server/discover` probe is no such message: rmcp answers it and stays
open for either protocol era.

The shutdown path runs once the service ends, whatever ended it: the channel is told
we are going away and every in-flight eval is cancelled and reaped, so a disconnect
mid-eval leaks no process tree.

## const CLOSE_FLUSH_GRACE
Bounded because a peer that has already gone must never hold the shutdown open.

## fn close_channel_for_shutdown
Without this a shutting-down host simply VANISHES and the agent sees a bare 1006,
indistinguishable from a crashed host - the very distinction the explicit close frame
exists to provide.

The frame is written by the hub task, so the wait is an ACK rather than a sleep: the
hub fires the completion AFTER its flush. A sleep long enough to usually work would be a
race dressed as a guarantee.

## fn spawn_signal_sweep
EXITING IS PART OF THE CONTRACT: handling a termination signal without terminating
would make the host ignore the very thing it was told to do.

It is also what makes the handler OBSERVABLE. A swept host reports
`code() == Some(128 + signo)`, while a host that merely took the signal's default
action reports `code() == None` and `signal() == Some(signo)`. Only the former proves
the sweep ran, which is what the system test measures.
