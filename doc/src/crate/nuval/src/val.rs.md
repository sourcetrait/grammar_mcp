# val.rs

The value model began as User's prototype in `sourcetrait_nuin`,
`src/val/data.rs`, carried verbatim. `Val` is named `NuVal` so it imports
without colliding with the many crates that name a value `Val` (User);
`ValError` and `ValResult` follow it.

## type NuValResult
A 1:1 of the serializable `nu_protocol::Value`, split in two: every value but
`Value::Error` is `Ok`, and the error is the `Err`, so an error travels beside a
value as a result rather than inside it as one.

## enum NuVal
nushell's `Value` less three variants. `Error` is the result's other half.
`Closure` holds a block and captures that belong to the engine that made them,
and `Custom` a plugin's own type, so neither means anything outside its engine
and neither is carried. `Record` keeps its pairs in order, as nushell's record
does.

## const SPAN
A value carried as data has no span, its source gone, so a member converted
back to nushell takes the unknown span.
