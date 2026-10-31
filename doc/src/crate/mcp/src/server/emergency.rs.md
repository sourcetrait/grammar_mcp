# emergency.rs

The host's emergency lane: the operator's emergencies and the Channel's spam pair, each
logged, then announced. The conditions themselves are the operator's model (the
operator's `emergency.rs`).

## type EmergencyTx
Unbounded so no producer ever blocks or back-pressures - the operator's router and the
face's pump, which carries the Channel's spam pair, both push here, and the volume is
low because emission is edge-triggered.

## fn emergency_model
`mcp/` IS A RESERVATION. Every host-originated model lives beneath it, which is what
lets a model path from a FOREIGN source be checked mechanically - anything claiming
`mcp/` that is not the host is not entitled to it. Enforced today at the one place a
non-host picks a model, and available to a future peer surface on the same terms.

A unit test asserts EVERY variant is under the reservation and that the models are
distinct. One variant escaping would turn a mechanical provenance check back into a
judgement.

## fn emergency_record
SHARED by the log line and the channel packet, so the durable record and the
notification can never disagree about what a condition reported. That sharing is also
what made a `from` event field easy to get wrong: the packet's envelope already carries
one, which is why the spam pair names its offender `origin`.

## fn emergency_line
Every field is a flat scalar, but a string field can still carry a newline - a spam
error's `action` holds an error's message - and `to nuon` writes a newline inside a
string raw. The render therefore goes through `escape_line`, as a packet's does, so
every record of `emergency.nuonl` stays on one line; the escapes read back as the
original characters, a compact render's raw newlines sitting only inside quoted
strings.

## fn emergency_log_path
`mcp_nom` namespaces it per PROCESS because the cache root is only
`(id, namespace)`-scoped, so two concurrent hosts on one namespace would otherwise
clobber each other's log.

## fn append_line
Open-append-close per line: low volume, and robust against an external truncation
between writes. Nothing is buffered to lose.

## fn announce
Through the Channel's gated emit (`EmitRequest`), so it is VERIFICATION-GATED like any
other telemetry - an unproven peer must not receive host state. A closed or unverified
channel silently DROPS the announcement, which is correct rather than an error: channels
are optional and the log already holds the record. The Channel is the running face's,
reached through the bridge (`server/channel/bridge.rs`) until EmergencyPolicy makes
the responder its controller's.

No rate limiting is applied or needed. The operator's level gates upstream already bound
each condition to roughly one report per episode.

## fn spawn_emergency_responder
The log is written FIRST and unconditionally, so the durable record never depends on a
channel being open. The task ends when every producer drops the sender.
