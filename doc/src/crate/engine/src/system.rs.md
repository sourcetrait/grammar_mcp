# system.rs

The engine as a subsys system: a host starts it, requests through its handle, and stops it.

## struct EngineSystem
Its loop only receives. Each request is answered on a task of its own, so a long eval or
a lock wait holds no other request up, and the loop is always free to take a stop.

## struct EngineSysParams
Unit, as `EngineSysConfig` is: the engine's own parameters arrive through `init`,
process-wide, so the system takes none of its own.

## struct EngineSysPaths
The namespace's homes as `init` set them: config, data, and state the data home, cache
and runtime the cache home, and both secrets dirs the keypair's. The engine keeps nothing
outside its namespace.

## fn on_channel_recv
The answer is sent from the request's own task, on a clone of the system's sender; the
loop has gone back to receiving by then.

## fn on_stop
A halt tears down every call in flight, as the `Teardown` command does, so a system
stopped mid-eval leaks no process tree.

## fn answer
Every request is answered, a refusal included, so a host never waits on one the engine
dropped.

## fn contained
A panic in a request's work answers as `internal` on that request rather than ending the
system. An eval's own panic never gets this far, its thread catching it; this is for the
engine's code around the eval, the equip lifecycle's among it.

## struct Unawaited
The handler for a packet no request waits on. The engine sends nothing but answers, so
none arrives; one that did would go back unhandled.

## fn EngineHandle::start
The system starts on the namespace `init` set, so `init` comes first, and the equip store
before both (`serve.rs`'s `EngineCore`).

## fn EngineHandle::request
Generic over the request types, each naming its answer, so a host gets the answer's own
type back: a `ValueAnswer` for an eval, an `EngineResult<ControlAnswer>` for a command. A
request the system could not take or answer is `internal`, the system being gone.

## fn EngineHandle::control
A command to the local engine, the common case, its delivery's failure folded into the
command's own result.

## fn EngineHandle::stop
A HALT, not a drain: the calls in flight are torn down, and the stop returns once the
system and its control task have ended.
