# msg.rs

The engine's protocol: what a host asks of it and what it answers. Every type is plain data
(`cereal`'s `Data`), serializable whole, which is what lets a request name a linked host's
engine at all.

## enum Host
`Local` is the engine in this process; `Remote` a linked host by its nom, the identity its
Channel and its remote links already go by.

## struct ExecuteRequest
The parameters as the agent sent them, the data and the timeout still text: the engine
holds them to the contract, so a host never parses what the engine will check.

## struct CallRequest
As `ExecuteRequest`, the data and the timeout still text: the engine holds them to the
contract its rig index records for the function's `main`.

## enum EngineCommand
Everything a host asks that isn't an eval, so one request type reaches the engine's state:
its calls in flight, its health, its equips, and its documentation.

## struct ValueAnswer
The nonce rides back with the value or the refusal: it is the envelope's either way.

## enum ControlAnswer
`Uninstalled` carries no equip when none was installed, and that is a success: absent is
the state asked for.

## struct ProcessRow
`source_nonce` and `path` are per-kind, absent where the kind has none, so the row a host
renders says by its shape which tool made the call.

## struct HungEval
A watched thread as the supervisor needs it: its lane tells a held stateless permit from a
stuck stateful lane, and its cancel time is what the grace is measured from.

## enum ToEngineSys
The system's inbound messages. Each request type converts into its variant by `From`, and
names its answer type, which converts back out of `FromEngineSys` by `TryFrom`; an answer
of the other variant is subsys's `ResponseType`, its error for a reply of the wrong kind.
