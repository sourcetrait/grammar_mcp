# client.rs

The serving MCP of every build. The bare entry serves through it with or
without `embedded-cli` and under `--test` alike; only `test-driver` reaches the
in-process engine (`run.rs`, `server/run.rs`).

## struct ClientSh
The frontend's state alone: the nonce generator, the interface parse's engine,
and the Channel's face. It holds no engine handle, so no path from here can
start one or make engine or equip state.

### field interface_engine
Built from the deny set as the test driver's is, so a denied command is unknown
to the parse on either entry and a malformed command is refused with the same
diagnostics.

### field face
The frontend-owned Channel commands, `open`, `ready`, and `close`, on the
process's one Channel through the face the test driver uses too
(`server/channel/face.rs`).

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
The ownership is `doc/ai/daemon/runtime_ownership.tsv`'s. `channel open`,
`ready`, and `close` are the frontend's and run here. `channel sync` and
`channel config` are the runtime's, as every other command is, and answer
unavailable: a sync-mode Channel opens but cannot be claimed until a runtime
holds its queue.

## fn ClientSh::interface
The command meets the same contract parse as on the test driver, so a
malformed one is refused `interface::invalid` before its ownership is read.

## fn ClientSh::get_info
The instructions name the namespace, as the test driver's do, and say that the
runtime's operations answer unavailable while no runtime endpoint is
configured.

## fn serve_client
The scratch directories are made first, because the Channel's inbox goes beside
the transient one (`scratch.rs`); no engine, substrate, or equip directory is
made. The signal sweep has no server shutdown of its own to run.
