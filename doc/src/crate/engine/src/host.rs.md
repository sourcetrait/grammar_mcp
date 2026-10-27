# host.rs

## trait EngineHost
What an eval asks of the process serving it. The Channel, the remote links, the
configuration, and the pins are the host's, and the engine reaches them only here.
The calls run on the eval's thread inside a synchronous nushell command and wait
for their answer, which a component's packet cannot give back to a waiting thread,
so the host is a trait the engine holds rather than a component it messages.

An answer is a message id, or the reason in the host's words: the decl puts it in
its error's title, where the envelope's message carries it.

## struct ChannelSend
`stop` is the engine's half of the spam gate. The host decides the hard threshold,
and stopping the offender, its signals and its job, acts on the eval's own state,
so the host calls back for it and reports the action taken.

## struct RemoteSend
`attached` passes as the body gave it, the files table, so the host reads it in the
order a send always has: each dest checked and each file read before the id is
minted.

## fn host
An engine nothing started still parses and evaluates; a `grimm` send or pin there
is refused rather than a panic, and the config reads as an empty record.
