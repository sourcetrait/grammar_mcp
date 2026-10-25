# commit.rs

## fn commit
A builtin's bare name resolves to its equip first, so `commit("gram")` commits
`sourcetrait/gram`.

Takes the equip's WRITE lock, since a commit wipes and rebuilds the canonical subtree, both
kinds of it, and a concurrent call reading the rig's index would see it mid-flight.

The engine comes from `lint_engine.current()` rather than a stored handle, so the validator
picks up a plugin registered since the host started. A validator that never refreshed would
reject source that RUNS.
