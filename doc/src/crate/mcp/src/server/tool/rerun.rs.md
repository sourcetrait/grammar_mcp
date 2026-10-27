# rerun.rs

## fn rerun
`nu` again on an earlier call's definition with fresh data: the same parameters as `nu`
less the definition, which the nonce names.

THE `is_base62` GATE IS PATH-TRAVERSAL DEFENSE IN DEPTH. The nonce is joined onto a
filesystem path, so it must reject `..`, `/` and anything outside `[0-9a-zA-Z]` before
that join happens.

The definition is ANALYZED AGAIN, never trusted as cached: its types are what the fresh
data and the result are held to, and the lane is its own `--env`. NO RE-LINT: the
original call already linted it, and the cache is the host's, so a lint could only refuse
something the host itself wrote.

A FRESH nonce is minted, and the rerun caches the definition again under it, so every
`nu` call's nonce, a rerun's included, is uniformly a handle, and the calls' logs stay
apart.

`InFlightKind::Rerun` carries the SOURCE nonce so `processes()` can show which definition
a rerun is re-firing, the one thing a caller cannot infer from the fresh nonce.
