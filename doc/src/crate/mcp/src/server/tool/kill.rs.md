# kill.rs

## fn process_kill
The cancel and its escalation are the engine's `Kill` (the engine's `serve.rs`). An
unknown or already-finished nonce is a no-op there, so the tool answers success either
way: the call is not running, which is the state asked for.
