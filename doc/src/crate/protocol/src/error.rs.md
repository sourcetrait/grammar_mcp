# error.rs

The runtime's and the transport's errors, apart from the engine's own. Each
renders to the frontend's envelope as rows of `Diagnostic`s, the same table every
failure has always carried, so a refusal reads alike whether this host or a worker
raised it.

## const LOCAL_HOST
`local` is hard-coded and every other alias is the configuration's, User's ruling,
so this host's alias can never be configured away.

## enum RuntimeError
What the runtime refuses outside the engine: an interface call's violations, an
unknown host, a schema, a purview, the Channel, and the retained remote Channel.

### fn kind
The violations variant `unreachable!()`s here deliberately: it renders through
`bucket`, and reaching this arm would mean a caller bypassed that path.

The remote kinds name their remote's kind, `remote::channel::`, as the commands
do, so a remote engine's sit apart under `remote::engine::` (User).

`remote::channel::open_failed` is the synchronous bind or connect failure of a
blocking `interface remote channel listen` or `connect`: distinct from the async
`remote/channel/Disconnected {error}` Channel model, which fires only for a
listener's post-bind accept failure or an established link's later teardown.

`remote::file` is the remotes file's own refusal, a file that doesn't parse or
holds a malformed entry, named with its path; a bad value a command was given is
that command's `remote::channel::invalid_params` instead.

An `interface` call's refusal is `interface::invalid` rows, gathered under
`InterfaceViolations`, each at its position in the command; a command that parsed
and ran refuses with its own kinds, as its tool did.

`remote::channel::unverified` refuses an `interface remote channel listen` or
`connect` while the Channel isn't verified, before any bind or connect: the link's
notices and relays ride the Channel's gate and would drop.

`channel::wrong_mode` names the mode the Channel is open in and the one asked for:
an open in the other mode, or a sync on an async Channel, each told to close
first. `channel::not_claimed`'s message follows the mode, a Monitor to start or a
sync to call.

`host::unknown` is a tool's `host` alias naming no host this MCP knows, its message
naming `local`. The engine's own `host::unserved` stays apart: that is a request
naming a host its engine doesn't serve, which no tool sends.

## enum TransportError
The path to a worker failing, never the worker's answer, each under a stable
`daemon::` kind. `AuthenticationFailed` means nothing was sent, while
`UnknownOutcome` means the request may have run, so it names the operation whose
status the caller can look up; an agent can tell a retry that is safe from one
that may repeat an effect. `Busy` admits no operation at all.
`UnsupportedFeature` is a hello's required feature the other end doesn't have
on; it sits last so the variants before it keep their wire indices.
