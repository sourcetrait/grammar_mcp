# remote_channel_open.rs

## fn remote_open
The Channel's phase is checked first: a link's Connected, Disconnected, Sent, and
Unsent notices and every relayed message ride the Channel's verified gate, so on an
unverified Channel they would drop silently. The refusal, `remote::channel_unverified`,
comes before any bind or connect.

Blocks on open_remote_blocking (link.rs) and returns synchronously: void on success, an
error envelope on failure. Three failures are an envelope - no such alias in
remotes.toml, a link already open for that alias, or the bind or connect itself failing
(remote::open_failed). A successful return means the immediate networking step
succeeded - a listener is bound, or a connector is connected - not merely that an open
started. A listener's peer-wait (Connected) and any established link's teardown
(Disconnected) still surface later on the Channel. The keys and addresses come from
the configured remotes file, so the command takes no address or key of its own.
