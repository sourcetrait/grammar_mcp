# msg.rs

The engine's half of its protocol. The requests, commands, and answers are the
protocol crate's, portable data every endpoint shares; what stays here needs the
engine: each request names its answer type for subsys against `EngineSystem`,
which the protocol crate never sees.

## impl subsys::Request
`ExecuteRequest`, `RerunRequest`, and `CallRequest` answer with a `ValueAnswer`,
`ControlRequest` with an `EngineResult<ControlAnswer>`. The trait is subsys's and
the request types are the protocol's, so the impls stand here because
`EngineSystem` is this crate's: the orphan rule admits an impl whose trait
parameter is local.
