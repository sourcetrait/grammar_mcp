# run.rs

## fn run_server
The test driver's server, reached by `grammar-mcp test-driver` under
`embedded-cli` alone (`run.rs`); the serving MCP is the client
(`server/client.rs`). The order here is load-bearing, top to bottom.

`install_child_subreaper` comes first so an eval's orphaned grandchildren stay on
our /proc ppid chain for the tree-kill. Everything after it can spawn externals.

The controller starts after the substrate (`server/controller.rs`): its engine
hydrates the equips' locks from the store the substrate makes, builtins included,
so an engine started first would answer a call to a builtin
`equip::not_registered`. It starts the face's systems first, then the engine, the
operator, which takes the namespace's host lock, and the observer, supervising.
The `mcp_nom` is the process's (`process_identity`, `server/identity.rs`), and the
face, the operator, the emergency log, and `NuSh` all carry it.

The signal sweep is installed before serving, its shutdown the controller's.
Without it a signalled host drops its eval children and its background jobs onto
the box unreaped, because the stdio path only ever learns about a clean client
disconnect. None of it reaches SIGKILL - that is what the host lock is for, and
the next host's sweep of the scratch directories (`scratch.rs`).

Nothing from outside panics it. The substrate, the controller's systems, the
stdio handshake, and the service's join each come back as an `Err` naming what
failed, which `run_test_driver` prints and exits 1 on. rmcp's `serve` refuses a
first message it cannot take, a notification before any request, and that is the
client's error to report, never a host panic. Claude Code's `server/discover`
probe is no such message: rmcp answers it and stays open for either protocol era.

The shutdown runs once the service ends, whatever ended it: the controller tells
the Channel's peer it is going away, then stops its systems in order, every
in-flight eval cancelled and reaped first, so a disconnect mid-eval leaks no
process tree.
