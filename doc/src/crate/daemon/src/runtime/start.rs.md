# start.rs

## static STARTED
One runtime a process, and only once: the engine underneath holds process-wide
state that initializes once and never again, so a second runtime, under
another identity or the same one after a shutdown, can't own an engine of its
own. The claim is an atomic swap, so two starts racing on two threads leave
exactly one runtime.

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

## fn shutdown
Consumes the runtime and leaves the process's claim taken, matching the
engine's single initialization.
