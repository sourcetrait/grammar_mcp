# config_channel.rs

## struct ConfigChannelParams
A PARTIAL update - supply only what should move - so a caller adjusting one threshold does
not have to restate the other three and risk reverting a change it did not make.

Windows are INTEGER SECONDS because the interface's arguments are strings and whole numbers
alone, literals the contract reads without evaluating (`server/interface/contract.rs`), so a
nu `duration` has no shape there. The `10sec` form lives in the model and the docs, never in
a command.

## struct ConfigChannelResult
Returns what is in force WHETHER OR NOT this call changed it, so a caller never has to assume
its own update took.

## fn channel_config
THRESHOLDS ONLY. The port and the cert dir cannot change under a live hub, so they stay set
once at startup - offering them here would be a knob that silently does nothing.
