# face.rs

The Channel's and the remote channels' commands and the Channel's shutdown,
shared by the client and the test driver, so both entries serve them from one
piece of code.

## struct ChannelFace
What a server needs of the Channel: the process's one handle, the lock that
serializes an open, the identity its greeting carries, and the remote links.
Both servers serve every Channel command and every remote channel command
through it (`server/client.rs`, `server/tool/interface.rs`).

### field channel
The process-wide handle, NOT a fresh one. An eval reaches the channel through
the same global, so a per-server handle would diverge from what bodies actually
emit on.

### field channel_open_lock
ASYNC because it is held across an await; the channel's own state lock is a std
Mutex precisely because the emit path must never need a runtime handle. The two
locks differ in kind for opposite reasons, which is worth not "simplifying".

### field remote_links
A clone of link.rs's process-wide registry, so the commands and the grimm sends
see one map. It is keyed by alias in both roles: a listener registers its link
under the alias it opened, as a connector does.

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

The server's own shutdown is the caller's closure: the test driver's stops
mcp's system, and the client has nothing to stop. The signal's number is
tokio's raw value, so the portable frontend takes no nix.

The scratch directories go last, after the components that write into them have
stopped: `process::exit` never returns to `serve_or_oneshot`, whose ordinary end
would remove them otherwise. The sweep is Unix's; elsewhere none is installed,
and the ordinary end does its work.
