# limits.rs

The wire contract's frame ceilings and decode budget. The other limits it sets,
the deadlines, the concurrency, and the backlogs, belong to the tasks that
enforce them.

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
Status takes RPC's ceilings, the contract naming none of its own. It is cereal
data because the hello carries it.

## type DecodeBudget
bitcode's `Budget` from sourcetrait's fork, so the bounds a payload decodes
within are the ones its decoders charge, each checked before what it bounds
is allocated. The heap is what the decoded value keeps; the decoder's own
column buffers, freed when the decode ends, are outside it.

## const DECODE_BUDGET_DEFAULT
The contract's 100,000 decoded elements and 32 MiB of decoded heap. Its depth
is 128 where the contract names 64: the decode counts every collection that
nests with values in it, a frame's own among them, so a value's 64 levels fit
inside a frame's containers. Those 64 are `check_value`'s, held at the
daemon's dispatch before a frame is dispatched, since a decode counting every
container can't count one type's levels apart. 128 levels of decoding take
well under a two-MiB worker stack.

## const DECODE_HEAP_HARD
The heap a negotiated budget may reach, the contract's ceiling.
