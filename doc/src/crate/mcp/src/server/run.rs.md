# run.rs

## fn run_server
The test driver's server, reached by `grammar-mcp test-driver` under
`embedded-cli` alone (`run.rs`); the serving MCP is the client
(`server/client.rs`). The order here is load-bearing, top to bottom.

`install_child_subreaper` comes first so an eval's orphaned grandchildren stay on
our /proc ppid chain for the tree-kill. Everything after it can spawn externals.

The face starts the Channel's and the peer links' systems before mcp's system
does (`server/channel/face.rs`), so the emergency lane mcp's system builds finds
the face's hub slot to install into, and mcp's system's controller is handed
both, the route the engine's host asks take (`server/host.rs`'s `HostRoute`).

Mcp's system starts after the substrate: its engine hydrates the equips' locks
from the store the substrate makes, builtins included, so an engine started first
would answer a call to a builtin `equip::not_registered`. It brings up the
emergency lane, the operator, which takes the namespace's host lock, and the
observer with the engine (`server/system.rs`). The `mcp_nom` is the process's,
drawn for the engine's start (`process_identity`, `server/identity.rs`), and the
system's operator and emergency log and `NuSh` all carry it.

The signal sweep is installed before serving (`server/channel/face.rs`), its
shutdown mcp's system's. Without it a signalled host drops its eval children and
its background jobs onto the box unreaped, because the stdio path only ever
learns about a clean client disconnect. None of it reaches SIGKILL - that is
what the host lock is for, and the next host's sweep of the scratch directories
(`scratch.rs`).

Nothing from outside panics it. The substrate, mcp's system and its components,
the stdio handshake, and the service's join each come back as an `Err` naming
what failed, which `run_test_driver` prints and exits 1 on. rmcp's `serve`
refuses a first message it cannot take, a notification before any request, and
that is the client's error to report, never a host panic. Claude Code's
`server/discover` probe is no such message: rmcp answers it and stays open for
either protocol era.

The shutdown path runs once the service ends, whatever ended it: the channel is
told we are going away (`ChannelCloser::close_for_shutdown`), then mcp's system stops
its components in order, every in-flight eval cancelled and reaped first, so a
disconnect mid-eval leaks no process tree.
