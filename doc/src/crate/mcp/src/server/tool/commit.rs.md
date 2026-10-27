# commit.rs

## fn equip_commit
A builtin's bare name resolves to its equip first, so `interface equip commit gram` commits
`sourcetrait/gram`.

Takes the equip's WRITE lock, since a commit wipes and rebuilds the canonical subtree, both
kinds of it, and a concurrent call reading the rig's index would see it mid-flight.

The engine comes from `lint_engine.current()` rather than a stored handle, so the validator
picks up a plugin registered since the host started. A validator that never refreshed would
reject source that RUNS.

The result is the commit's own, its changed paths and the validator's `warnings`; a
refusal's error table carries its errors alone.
