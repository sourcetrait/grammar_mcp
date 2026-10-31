# face.rs

The Channel's and the remote channels' commands and the Channel's shutdown,
shared by the client and the test driver, so both entries serve them from one
piece of code. The face is the Channel's controller: it starts the Channel's
system (`crate/channel`) and holds it, binds each async open's hub, keeps the
open's verify timer, and pumps the system's lines into the hub.

## struct ChannelFace
What a server needs of the Channel: its system, the open's hub, the lock that
serializes an open, the identity its greeting carries, and the remote links.
Both servers serve every Channel command and every remote channel command
through it (`server/client.rs`, `server/tool/interface.rs`).

### field channel
The system this face started. An eval reaches the same one through the bridge
(`bridge.rs`), which the face's start fills, so what a body emits on is what
the face serves.

### field channel_open_lock
Async because it is held across the open's requests and its hub's bind.

### field remote_links
A clone of link.rs's process-wide registry, so the commands and the grimm sends
see one map. It is keyed by alias in both roles: a listener registers its link
under the alias it opened, as a connector does.

## fn ChannelFace::start
The system starts on the config's thresholds, its handler the face's pump, and
the bridge takes it at once, so the code not yet handed the face reaches the
Channel the agent opens.

## struct Hub
One async open's hub as the face holds it: its epoch, its endpoint, the queue
its peer task writes to the socket, the close lane, and the shutdown sender,
whose drop ends the accept loop.

## struct HubSlot
The face's side of the open, shared with its pump and its timer: the hub, the
armed verify timer, the emergency lane, and the epochs the face mints.

### fn next_epoch
From 1, so the Channel's closed epoch, 0, is never an open's.

### fn peer_gone
The peer task drops its queue's receiver when it ends, so a closed queue is the
peer gone, read the moment it happens, before the system hears the notice.

### fn deliver
A line for another open's epoch is dropped: the hub it was for is gone.

### fn close
The close signal does not share the packet queue. A close queued behind traffic
could arrive late or not at all, and without an explicit close frame the agent
sees a bare 1006 - indistinguishable from a crash. Dropping the hub drops its
shutdown sender, which ends its accept loop, and the open's verify timer stands
down with it.

### fn close_now
The completion half exists for shutdown. The frame is written by the hub task
asynchronously, so a caller that exits the process immediately after asking for
a close races that write and leaves the peer with exactly the bare 1006 the
explicit close exists to prevent. Awaiting an ack makes the ordering
deterministic instead of a sleep long enough to usually work.

### fn arm_verify
Holding the sender is what keeps the timer armed and dropping it is the cancel,
so verification, a close and a re-arm all stand the timer down. That is also
why re-opening cannot leave two timers racing one channel. The epoch rides with
the sender, so standing one open's timer down leaves another's armed.

## struct HubPump
The face's handler of its Channel's system. It does its work before it returns,
on the control task, so the system's next answer reaches its waiter only after
the line is queued or the frame signalled.

## const CLOSE_FLUSH_GRACE
Bounded because a peer that has already gone must never hold the shutdown open.

## fn ChannelCloser::close_for_shutdown
Without this a shutting-down host simply vanishes and the agent sees a bare
1006, indistinguishable from a crashed host - the very distinction the explicit
close frame exists to provide.

The hub's frame is signalled first, then the logical Channel closes, so the
system's close finds no hub left to signal again; the ack is awaited last. The
frame is written by the hub task, so the wait is an ack rather than a sleep: the
hub fires the completion after its flush. A sleep long enough to usually work
would be a race dressed as a guarantee.

## fn spawn_signal_sweep
Exiting is part of the contract: handling a termination signal without
terminating would make the host ignore the very thing it was told to do.

It is also what makes the handler observable. A swept host reports
`code() == Some(128 + signo)`, while a host that merely took the signal's
default action reports `code() == None` and `signal() == Some(signo)`. Only the
former proves the sweep ran, which is what the system test measures.

The Channel closes first, then the server's own shutdown runs: the test
driver's stops mcp's system, and the client has nothing to stop. The signal's
number is tokio's raw value, so the portable frontend takes no nix.

The scratch directories go last, after the components that write into them have
stopped: `process::exit` never returns to `serve_or_oneshot`, whose ordinary end
would remove them otherwise. The sweep is Unix's; elsewhere none is installed,
and the ordinary end does its work.
