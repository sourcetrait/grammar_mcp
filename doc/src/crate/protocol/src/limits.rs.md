# limits.rs

The wire contract's frame ceilings. The other limits it sets, the deadlines,
the concurrency, the backlogs, and the decoded budgets, belong to the tasks
that enforce them.

## const HELLO_FRAME_MAX
Every stream's first frame is its hello, before any role is known, so every
stream starts under this ceiling.

## const RPC_FRAME_DEFAULT
## const RPC_FRAME_HARD
The default until the handshake negotiates, and the ceiling no negotiation
passes: a client asks for less, never more.

## const EVENT_FRAME_MAX
Events stay small, their default and their hard ceiling the same, so a slow
subscriber is a gap or a disconnect, never a backlog of large frames.

## enum StreamRole
The three roles the contract names, fixed for a stream's life by its hello.
Status takes RPC's ceilings, the contract naming none of its own.
