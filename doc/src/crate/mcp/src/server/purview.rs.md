# purview.rs

The session's side of the purviews. The stored rows and their resolution are the
engine's (`crate/engine/src/equip/purview.rs`), reached through its requests;
what stays here is the view each session holds.

## fn pattern_delta
What a change of view revealed, from the two resolutions' expanded patterns, so
a view change answers the signatures newly in view alone.

## struct CurrentPurview
Session-resident by design - it is a view rather than a configuration, so it belongs in
memory and dies with the host. Confirmed across a real restart, where the configured rows
persisted and the view reset.

### fn set
An empty list means `default`, so the view always names at least one purview and there is
no looking-at-nothing state to reason about - which is why there is no separate reset:
resetting is just setting the view to nothing in particular.

### fn retain_known
So a rig uninstall or a purview deletion cannot leave the session pointing at something
gone. The rows are the engine's answer to the write that changed them.

## fn unknown_purview
The refusal a view naming an id no purview answers to has always given.

## fn NuSh::resolve_purviews
Every view's resolution is the engine's, one request for a set of ids, its failure
answering in the command's envelope as the read's always did.
