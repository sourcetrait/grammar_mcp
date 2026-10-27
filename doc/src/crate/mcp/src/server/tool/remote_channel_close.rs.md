# remote_channel_close.rs

## fn remote_close
Removes the link from the registry and cancels its drivers; it does NOT wait or emit.
The link's own open task (link.rs) observes the cancel, deregisters if still current,
and emits mcp/remote/Disconnected - so a close and the Disconnected notice share the
one lifecycle path rather than each reporting separately.

An alias with no established link may name a listener still awaiting its peer:
close_listener (link.rs) cancels its accept and returns once its port is freed, and the
accept task emits Disconnected {remote}. not_open only when the alias names neither.
