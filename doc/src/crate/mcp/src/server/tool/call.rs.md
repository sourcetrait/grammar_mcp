# call.rs

## fn call
The arguments cross to the engine as a `NuVal`, converted from the agent's JSON object
through the crate's one JSON-to-Value path, so the request is plain data whichever engine
serves it. The engine resolves the namepath, takes the equip's lock, and runs the call
(the engine's `serve.rs`).

THE RESULT IS LENIENT to its last step: the engine answers nothing, a record, a table, or
`{}`, and a value NUON then cannot render answers `{}` here, where the render happens.
