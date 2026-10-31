# interface.rs

The `interface` tool: every tool but `execute`, `rerun`, and `call`, folded into one
`command` string (User's `grammar/interface`, its parameters his `doc/human/tools.md`).
The commands convert those tools as they were: same parameters, same results,
same refusals. The parse is the interface crate's (`crate/interface`); this
host binds it to its deny set and routes what it parses.

## const INTERFACE_PRETEXT
User's `include/human/mcp/interface.md`, verbatim by `include_str!`: his pretext
and nothing else, the command list being the code's to render.

## fn interface_description
His pretext, its trailing whitespace trimmed so one blank line parts it from the
list whatever his file ends with, then the commands the deny set leaves, by
their words under their topics, rendered from the catalog the parse registers.
The signatures moved to `interface help interface` and the grammar-mcp skill,
keeping the description small as the commands grow (User).

## const INTERFACE_DESCRIPTION
The router's stub, which `interface_description` replaces on the client's
router and the test driver's alike, so both carry the same description.

## fn interface_engine_for
The interface crate's engine, its commands the deny set leaves, the deny set
naming a command by its words.

## fn ChannelFace::run_channel / fn ChannelFace::run_peer
The routing table's two groups both servers share: the Channel's commands and
the remote channels' run through the face that holds their systems, so the
client and the test driver serve them from one piece of code.

## fn interface
The nonce is minted first, over the tool's name and the command text, so a
refusal at the parse carries one exactly as a dispatched command's answer does.
The host is resolved before the parse (`resolve_host`, `common.rs`); every command
runs on this host, `local` the one alias known. The dispatched command answers
under that same nonce: each command's method takes the nonce rather than minting
its own.

## fn run_interface
Each group to its system: the face's two, and the engine's to `run_engine`. No
command dispatches an eval, so none appears in `interface process list`'s table
and none takes a lane or a permit; a command's own locks are taken inside its
method as its tool took them. It is the test driver's: the client routes the
face's groups the same and answers the engine's unavailable
(`server/client.rs`).

## fn run_engine
The engine's group, each command its method, the purviews', `info`'s, and
`help`'s with the session's view.
