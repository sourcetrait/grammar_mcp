# hub.rs

## const BIND
The channel is a single-host design, so "only localhost gets in" is a property
of the socket rather than a policy to enforce - the kernel will not route
anything else to a 127.0.0.1 listener. That is why there is no peer allow list,
deny list, origin check or per-connection CA pinning anywhere in this file.

A literal rather than `localhost` also removes the v4/v6 resolution ambiguity
that can make a v4 reset read as a TLS verdict; the leaf carries IP SANs for
this.

## const FROM_MCP
Plain `mcp`, not `mcp/<id>`: the channel is 1:1, so there is no second MCP to
tell apart.

## fn open_packet
Built in one place because it has two emitters: the hub greets a freshly
connected peer with it, and `interface channel open` re-sends it on an existing
channel, where a failing send is what reveals a peer that has actually gone.

## fn server_config
The certificate and key are the host's `[identity]`, the same pair its remote
links present (`config.rs`). A host whose file names none refuses the open with
the table's name, so the refusal says what to add rather than failing on a file
nobody named.

The ring provider is passed explicitly rather than taken from the process
default. rustls defaults to aws_lc_rs while nushell already compiles against
ring, so naming it keeps the hub independent of whatever else may have installed
a default - and keeps a second crypto backend out of the dependency graph.

`with_no_client_auth` is deliberate, not an omission. The peer is authenticated
by the handshake over the already-trusted stdio session, which is all the
agent's Monitor can support: its input is `{url, protocols}` and carries no
headers, no client cert and no auth hook.

## fn start
Ephemeral port by default. The agent always learns the endpoint from
`interface channel open`'s return, so a fixed port buys nothing and costs a
squatting failure mode - a stale host or an escaped child holding it makes the
next open fail with a raw `Address already in use`.

The port is read back from `local_addr` rather than trusted from config: with 0
it is the only way to know it, and with a pinned one it confirms what was
actually bound.

## fn accept_loop
First connection claims, via `compare_exchange` rather than a load followed by a
store, so two simultaneous connections cannot both read "unclaimed" and both
win.

Dropping the shutdown sender counts as shutdown, which is why a torn-down
channel ends this loop even though nothing is ever sent on that oneshot.

## fn serve_peer
The winner receives `channel/Open` immediately, because the verification step is
that the agent sees a real packet - there is no separate ack channel by design.

The loop is `biased;` and the arm order is load-bearing. `close_locked` signals
the close and drops the packet sender in the same breath, so the close arm and
the `None` arm go ready together - and `tokio::select!` picks among ready arms
at random unless told otherwise. The `None` arm breaks the loop without writing
a frame, so without the bias a planned close would carry our code and reason
only about half the time and otherwise leave the bare 1006 the lanes exist to
prevent. Biasing makes the precedence structural instead of probabilistic, and
states the design directly: a planned close must never wait behind queued
traffic.

The race is nondeterministic, so one observed clean close is not evidence the
mechanism works - a nondeterministic path needs either repetition or a
structural argument, and the bias supplies the structural one.

Polling the read half is not optional: it is what lets tungstenite answer pings
and observe the peer's own close.

The frame guard lives here, at the last point before the wire. Above the cap the
client drops the frame whole and closes the watch; exactly at it the frame
arrives missing its first byte, which is silent corruption rather than an error.
So the bound is a hard `<` and nothing may ever be sized onto 1 MiB.

## fn refuse_peer
The close code and reason both reach the client verbatim, which was verified, so
a refusal is legible rather than looking like a crash. The refusal is the design
working, not something to route around.
