# call.rs

## fn call
The host is resolved right after the nonce (`resolve_host`, `common.rs`). The data and
the timeout cross to the engine as the agent's text, as `execute`'s do, so the request is
plain data whichever engine serves it and no host parses what the engine will check. The engine resolves the namepath, takes the equip's lock, holds the data to
`main`'s contract, and runs the call (the engine's `serve.rs`).

The result is strict, as `execute`'s is: the engine answers what `main`'s return type
holds or refuses it as `result::invalid`, so the render here meets only a value the
contract has already passed.
