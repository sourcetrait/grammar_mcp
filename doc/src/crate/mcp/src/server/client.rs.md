# client.rs

The serving MCP of every build. The bare entry serves through it with or
without `embedded-cli` and under `--test` alike; only `test-driver` reaches the
in-process engine (`run.rs`, `server/run.rs`). A runtime-owned tool or interface
command answers the existing envelope with `daemon::unavailable` until the
native client reaches a runtime; serving never starts an engine or makes engine
or equip state.

## struct ClientSh
The frontend's state alone: the nonce generator, the interface parse's engine,
and the Channel's face. It holds no engine handle, so no path from here can
start one or make engine or equip state.

### field interface_engine
Built from the deny set as the test driver's is, so a denied command is unknown
to the parse on either entry and a malformed command is refused with the same
diagnostics.

### field face
The Channel's commands and the remote channels', on the Channel's system the
face started and the process's links, through the face the test driver uses
too (`server/channel/face.rs`).

## fn ClientSh::tool_router
The same four tools as the test driver's, with the same parameters and the
same descriptions, so an agent meets one surface whichever entry serves it; a
system test holds the two lists equal under `embedded-cli`.

## fn ClientSh::unavailable
A runtime-owned operation answers truthfully in the existing envelope, its
nonce minted first, then `daemon::unavailable` (`TransportError::Unavailable`).
No successful result is fabricated, and nothing is made on the way.

## fn ClientSh::runtime_owned
The host alias is checked before the runtime's answer, so an unknown alias is
refused `host::unknown` on either entry alike.

## fn ClientSh::run_interface
The ownership is `doc/ai/daemon/runtime_ownership.tsv`'s, after User's rulings:
the Channel is events and communication, and the remote engine is control. So
every Channel command and every remote channel command runs here, as the MCP
always served them, a sync Channel's queue and the thresholds among them; the
rest are the runtime's control and answer unavailable.

## fn ClientSh::interface
The command meets the same contract parse as on the test driver, so a
malformed one is refused `interface::invalid` before its ownership is read.

## fn ClientSh::get_info
The instructions name the namespace, as the test driver's do, and say that the
runtime's operations answer unavailable while no runtime endpoint is
configured.

## fn serve_client
The scratch directories are made first, because the Channel's inbox goes beside
the transient one (`scratch.rs`), and the cert profile is shipped as the test
driver ships it (`server/cert.rs`); no engine, substrate, or equip directory is
made. The face starts the Channel's system before the server is built, and its
closer is kept apart from the server the service takes, for the end and the
signal sweep. The signal sweep has no server shutdown of its own to run.
