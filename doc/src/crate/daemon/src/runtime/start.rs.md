# start.rs

## static STARTED
One runtime a process, and only once: the engine underneath holds process-wide
state that initializes once and never again, so a second runtime, under
another identity or the same one after a shutdown, can't own an engine of its
own. The claim is an atomic swap, so two starts racing on two threads leave
exactly one runtime. A start that fails before the engine starts gives the
claim back, so the process may try again; once the engine has started the
claim stays taken.

## struct RuntimeIdentity
The worker's identity is the effective UID with the MCP id and the namespace:
the UID is the security boundary, and same-UID namespaces separate state
alone. The identity is fixed at the start and never changes for the
runtime's life.

## struct Runtime
What an embedding host or grammard owns: started explicitly and shut down
explicitly, with no MCP server and no systemd activation in the way. Typed
dispatch and event subscription join it once the request and event types
exist.

### fn start
The order is the namespace's safety. The lock comes before anything that
touches the namespace, so a second worker is refused before its scratch, its
journal, its engine, or a plugin; then the generation is recorded in the lock,
the scratch made, the controller's systems started, the journal first, and the
engine's parameters, config dirs, and substrate set, once. A failure undoes
what the start made, the systems stopped, the scratch removed, and the lock
released with its descriptor, and leaves the namespace's state and equips as
they were; one before the engine gives the claim back.

### fn shutdown
The socket stops admitting first, then the systems stop, the journal's lock
let go, the tasks given a short grace, the socket removed, the scratch gone,
and the lock last, so a successor takes the namespace only once this worker is
done with it. The process's claim stays taken, matching the engine's single
initialization.

## fn tasks
The runtime's own threads, whether or not it serves a socket, for its systems
and its socket's tasks alike, so an embedding host needs no async runtime of
its own to start a worker. A runtime that can't be made is
`runtime::tasks_unavailable`.

## struct Serving
The socket's listener and its tasks' stop.

## fn serve
The socket is bound after the engine has started, so a stream is never admitted
by a worker that can't serve it. The worker's profile offers the instance
file's limits, or the protocol's defaults, each held to the protocol's hard
ceiling as a hello negotiates it. The sessions are timed by the worker's own
monotonic clock from the socket's start, and ticked beside the accept task.

## fn start_engine
The MCP's substrate, made as its host makes it: the equip store, the default
purview, the plugin registry, and the builtins on a fresh store. A default
purview that can't be made is reported, as the MCP's host reports it, and the
start goes on. The worker's nom is its own, drawn at each start.

## fn fresh_generation
The protocol's `WorkerGeneration`, sixteen random bytes: random rather than
counted, so no file has to survive a crash for a restart to be told apart from
the worker it follows.
