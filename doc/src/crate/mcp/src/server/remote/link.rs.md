# link.rs

The link lifecycle over the codec (codec.rs) and the verifier (verify.rs). One
link is two known-public-key mTLS connections to one peer, message and control
on one and files on the other, so file bulk never head-of-line-blocks control:
TCP is one ordered stream, so the separation is two connections rather than a
multiplex inside one. A link opens on demand, after each agent's Channel is
verified; the host runs no listener of its own.

## static REMOTE_LINKS
A std Mutex, not the async one, because a grimm send reaches it from the
synchronous eval thread. Its guards are held for map operations alone, never
across an await.

## fn connect_remote / fn listen_remote
Each blocks on the link's immediate networking result, so `interface remote
channel connect` and `listen` return synchronously; only a listener's peer-wait stays
async. Which role a host takes is the agent's call at each open, the remotes
file giving only the defaults (`remotes.rs`).

- connect_remote: await connect_link under CONNECT_TIMEOUT (20s). On success
  register the link and spawn only a teardown watcher (await both joins,
  deregister, Disconnected); there is no Connected for the attempt - the
  synchronous success is the notice. A connect error or timeout returns Err,
  surfaced as remote::channel::open_failed.
- listen_remote: await bind_listener (bind and build the acceptor, the sync
  half) and return the address bound, its port the kernel's when random, or the
  bind error as remote::channel::open_failed. Then spawn the async accept-serve:
  accept_and_pair one peer's two connections, register the link, emit Connected
  {remote, mcp_nom}, await both joins, deregister, emit Disconnected. An accept
  failure after a good bind emits Disconnected {remote, error} (post-bind, so
  async).

A connect attempt emits no Connected or Disconnected; its synchronous return is
the notice. Disconnected is kept for any established link's teardown, in both
roles: once told the link is up - Connected for a listener's peer, or the
synchronous connect for a connector - a later drop still emits Disconnected, so
a long-running consumer learns the live link died. deregister_if_ours drops the
registry entry only if it is still this task's link, since a later open may
have replaced it under the same alias. emit_open_failed (Disconnected {remote,
error}, mcp_nom absent) serves only the listener's post-bind accept failure; a
connect's and a bind's failures return synchronously.

## fn bind_listener
The bind is the address asked for, never another: a port taken is the agent's
to answer, a listen on the ip alone taking the kernel's choice (User).

## fn accept_and_pair
Every peer's connection is taken to the handshake, where the known-key match
decides it; no source address is filtered, the peer being known by its key.

`remote` (the opened alias) is always present on a Connected or Disconnected
packet, so a consumer keys on it and branches on field presence: mcp_nom
present iff the link established, error present iff it failed - absent when
not, never null.

## static BOUND_LISTENERS / fn close_listener / fn promote_listener / fn unlisten
The bound-but-unpaired listener registry, parallel to REMOTE_LINKS.
listen_remote registers a `BoundListener` the moment the bind succeeds, at the
address bound, so `interface remote channel list` can surface a listener that
has bound but not yet paired, and `interface remote channel close` can cancel
one.

An entry carries its address, a sequence number, its accept's cancel token, and
the receiving half of a `released` signal. close_listener removes the entry,
cancels the token, and awaits `released` up to CLOSE_FRAME_GRACE: the accept
task's `select!` (biased, the cancel first) drops the accept future, which drops
the TcpListener and frees the port, and only then sends `released`. So a close
returns with the port free, and an immediate re-open of the same alias binds
rather than meeting "address in use".

A pairing that completes as a close lands is settled under the registry's lock:
promote_listener moves the link into REMOTE_LINKS only while the alias still
holds this task's sequence number, so a close that removed the entry first
wins, and the paired link is cancelled there and then. Promotion registers the
link before the entry leaves, so it is briefly in both sets, never in neither.
The lock order is fixed: the listeners' map, then the links' map; `interface
remote channel close` never holds both.

unlisten, on an accept failure, leaves only this task's own entry, by its
sequence number, since a close followed by a re-open can put a newer listener
under the same alias.

A listener closed before it paired emits Disconnected {remote}: no `mcp_nom`,
since no link established, and no `error`, since nothing failed.

## fn find_link_send / fn safe_dest
find_link_send matches on the link's remote_mcp_nom (the id an agent sends to),
and holds the registry lock only for the lookup and the synchronous mpsc pushes.
safe_dest is the one guard for a dest (relative, no `..`, all Normal
components), applied sender-side, where it is a clean error, and receiver-side,
as defence against a buggy peer.

## struct RemoteLinkHandle
The join handles live in the open task rather than here, so a cancel
(`interface remote channel close`) lets that task observe the end and report it.

### fn enqueue_send
Registers the send in the SendTracker (its dests and the message) before any
push, so no write outcome can arrive for an unknown id; the pushes are
synchronous mpsc sends, so a grimm eval thread drives them. A mid-push failure
means the driver is gone (the link went down at enqueue): the registration is
forgotten and the error returned synchronously, with no id and no async notice,
since nothing left the host. push_frames is the free helper doing the pushes;
splitting it out keeps the register/forget bracket readable.

## struct SendTracker / struct PendingSend
The aggregate write-outcome half. A send is Sent only when the Deliver and every
file dest's last chunk have written OK across both connections (the whole
transmission), Unsent on the first write error or a link teardown mid-send. This
is not a peer round-trip: the transport write is the ack (TCP and TLS perform
it), so there is no DeliverAck and no receipt timeout. Per-id map: each driver
reports its writes (dest_written on a last-chunk write, message_written on the
Deliver write), completeness is checked under the mutex, and whichever write
finishes the set fires Sent exactly once (the map remove). failed() is
idempotent per id and cancels the shared token so the other connection winds
down; flush_unsent drains the map on each driver's exit, idempotent, so a
teardown reports every still-pending send as Unsent without double-firing. Files
ride the file connection and the Deliver the message connection, so aggregating
"the whole send wrote" needs this shared per-link tracker rather than a single
write site.

## the receive side (RecvContext, DeliverSlot, try_finalize, relay_to_channel)
A delivered message becomes a normal Channel packet (a relay, never an
"injection"). The slot-map per id coordinates the two connections: the Deliver
sets the model, the event, and the expected dest set, each file's last chunk
marks a dest done, and try_finalize fires once (the map remove), from whichever
driver completed the condition, when the Deliver is in and every dest has
landed, relaying through the verification-gated emit. The receiver sends
nothing back - its Sent already fired from the sender's write, and a refused
relay (an unverified Channel among the causes) is logged on the receiver alone,
never reported to the sender as Unsent. The slot-map is what makes the files
exist on disk before the agent observes the emission. land_file_chunk appends
under <inbox>/<peer_nom>/<id>/<dest> - the same inbox the Channel's own
attachments use, so a relayed packet's `attached` ref resolves for the local
agent. The inbox is read as each chunk lands, since every open makes a new
one: a closed Channel has none and lands nothing, and a delivery that spans a
close and an open loses its earlier chunks with the inbox the close removed.

relay_to_channel refuses a model under `mcp/` or `remote/` first, `reserved_model`, as the
sending grimm commands do (`reject_reserved_model`): the reservation is what
lets an agent take a model's path as proof of the host's origin, and a peer is
not the local host. Its files have landed by then and stay, as for any refused
relay.

rewrite_spill_pointer: a sender spills an oversized event to the reserved
EVENT_SPILL_DEST file with a dest-relative pointer; relay_to_channel rewrites
that path to <peer_nom>/<id>/<dest> before emitting, so the agent resolves it as
<inbox>/<spilled_event_path> uniformly with a local spill. A no-op for a normal
event or a path already carrying a `/`.

## fn push_report
Best effort: the local agent drove the send or the open, so its Channel is
verified, and a packet dropped on a channel closed since is no loss.

## the McpNom handshake
A Hello variant in each language (codec.rs), exchanged over the same Framed
read/write before the steady loop. The initiator sends first (its nom and the
stream role), the acceptor replies its nom. pair_check requires both connections
of a link to report the same peer nom - a mismatch means the two TCP connections
reached different hosts, which must never become one link. The nom crosses as
its base62 String (McpNom wraps a private Nonce(u64), not
bitcode-serializable).

## the driver loops
Two concrete driver fns per side (message and file), `tokio::select!` with
`biased;` and the cancel first, so a close beats an in-flight read
deterministically, the channel hub's close-race discipline. The In/Out enums and
the client-vs-server TlsStream types differ per side, so abstracting them behind
a trait costs more than the duplicated lines buy. close_initiator and
close_acceptor send Close and briefly await the reciprocal (CLOSE_FRAME_GRACE,
2s) so both ends see a clean shutdown rather than a bare drop.

## known-public-key mTLS config
Server authentication alone (`with_no_client_auth` on both sides) would bring a
mutual link up working and unauthenticated, so neither side uses it. Instead:
client = builder_with_provider(ring) .dangerous()
.with_custom_certificate_verifier(PublicKeyVerifier) .with_client_auth_cert(our
public key, our private key) - present ours and match the server's; server =
builder_with_provider(ring) .with_client_cert_verifier(PublicKeyVerifier)
.with_single_cert(ours) - present ours and require and match the client's. Ours
is the host's `[identity]`, the pair its Channel presents too (config.rs),
carried with the peer's in `LinkKeys`. The
ring provider is set by hand so the link never depends on a process default and
no second backend creeps in. PublicKeyVerifier implements both verifier traits
(verify.rs), so one value serves both sides. Single-leaf chain (load_chain =
vec![the public key]): the match uses the leaf DER and ignores any chain.
PLACEHOLDER_SERVER_NAME is a fixed `.invalid` placeholder - rustls needs a
syntactically valid ServerName even though the match ignores it, there being no
real DNS identity under known-public-key trust.

## runtime proof
The TLS lifecycle is exercised at runtime: the system tests' `remote_channel.rs`,
over the harness's `remote.rs`, link two real hosts over mTLS through a
simulated rmcp consumer and run an A->B->A echo - connect, accept, handshake, a
real send and relay, and the Connected, Sent, and Disconnected notice shapes.
Each host's one identity serves its Channel and its link alike. The unit tests
(the codec round-trip, the verifier match) stand alongside it.
