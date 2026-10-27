# template.rs

The two source builders, `nu`'s and `call`'s. Each embeds a NUON literal the host
rendered rather than leaving the eval to re-parse JSON, so the parser downstream only
ever sees well-formed nu. Neither sets `$env.NONCE`: `eval_in_process` puts it on the
eval's stack, where every command the source reaches finds it.

## const CALL_ALIAS
A constant WE own, and that ownership is the whole point. A call target's name is
the AGENT's to choose and may collide with a nushell KEYWORD, which can never be
shadowed - the keyword wins at parse under `use`, `overlay use` and
`overlay use --prefix` alike. `run` BECAME a keyword in 0.114 and there is a
`run` call target in the estate, so aliasing the overlay sidesteps the namespace
entirely and the drive is a constant rather than the target's own name.

## fn args_to_nuon

## fn json_object_to_nu_value

## fn json_value_to_nu_value
Also the write half of the rig index's NUON round-trip and the serde bridge every
tool's record crosses, which is why it is `pub(crate)` rather than private to the
templates. An out-of-range number degrades to a string rather than failing, on the
reasoning that a value the agent sent is better preserved lossily than rejected.

## fn build_call_source
Three pieces, each load-bearing under nu 0.114 and none of them obvious:

The overlaid path is the TARGET's own module under the equip's rig,
`equip/<author>/<name>/rig/...`, not the rig root. 0.114 (PR #18303) stopped
implicitly importing a module's submodules, so the pre-0.114 form - a root `use`
plus a module-qualified drive - binds nothing the drive can traverse and is dead.

`as __call` sidesteps the keyword collision described under `CALL_ALIAS`.

`--prefix` matters because a call target MAY export helpers beside `main`.
Without it the overlay flattens them bare into the eval scope, where one named
`open` would shadow the builtin for everything else in that eval.

Importing the target alone rather than the whole rig costs nothing: the target's
own `use equip/...` references bind at THAT file's parse against the const
`$NU_LIB_DIRS`, never through this import.

The literal is `null` whenever the args are empty, whatever the declared type: a call
is an authored definition, and its leniency is the author's.

## fn build_nu_source
The definition as the agent wrote it, a newline, then the one call. The newline is
load-bearing: a definition ending in a comment would otherwise swallow the call. The
contract has already held the definition to exactly one top-level `def execute`, so the
source holds nothing else that runs.

No wrapper scopes it. On the stateless pool the clone drops with its thread; on the
stateful lane `def --env` is what carries the body's `$env` and `cd` to the eval's top,
where the engine's merge persists them, the def's own `let`s and `def`s staying inside
it. The decl `execute` itself persists in the stateful engine, each call's later def
winning its own call's resolution.

The data's literal binds the `data` positional, so nushell's own check of it runs at
parse too, beneath the host's strict one; the timeout's literal follows only when the
signature takes `timeout?`.
