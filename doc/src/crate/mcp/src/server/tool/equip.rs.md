# equip.rs

## struct EquipParams
One parameter for all four commands, the source path: the namepath is read off it, so the
equip is wherever its source says it is.

## enum EquipSummary
UNTAGGED: the caller knows the variant from the command it ran, and the shapes are distinct.
`uninstall` returns no summary at all - void, matching its idempotent success.

`install`'s summary is its commit's result, the validator's warning rows in its `warnings`
as `interface equip commit`'s carry them. `check` keeps both buckets, `errors` and
`warnings`: it is the validator's report, so an error there is a finding rather than a
refusal.

## fn equip_install
An install brings a RIG into view only when the equip has one; a lib alone has no calls for
a purview to show.

THE INSTALL IS THE ENGINE'S, THE VIEW THE HOST'S, so the purview update follows the
engine's answer, outside the equip's lock (the engine's `serve.rs`). An uninstall landing
in between leaves nothing behind: the update's own prune judges by registration, and drops
the pattern it just added for a rig no longer there.

## fn equip_uninstall
An unknown equip returns success with no summary rather than an error, because absent IS the
requested state.

## fn purview_add_rig
The purview table is namespace meta beside the equip repository, and its load,
change, and save are not atomic, so it rides the repository's lock, as
`interface purview configure`'s write does.

ADDS, never replaces. `default` always carries a row - startup writes it as `['*']` - so the
first install simply appends beside it, giving `['*', 'my/rig:']`, and nothing leaves view.
Narrowing everything down to one rig as the price of installing it would be a surprising
trade.

The CURRENT view is a set of purview IDS rather than of patterns, so "add it to the current
purview" means adding it to each configured purview currently in view: `default` always, plus
whatever else the session named.

Every target HAS a row - startup materializes `default`, and any other id in the current view
was checked against the table to get there - which is why the lookup can be a plain `find`
with no fallback.

A BUILTIN seeds its own purview and its reference in `core`, then brings `@core` wherever any
other rig brings `<rig>:` - never into `core` or a builtin's own row, where it would only
make a cycle.

NON-FATAL: the install already succeeded and is not rolled back, so a bookkeeping failure is
reported to stderr rather than turned into a failed install. It is worth reading though,
because a `default` that failed to gain the pattern leaves the new rig installed but out of
view.

## fn purview_remove_rig
The rig's own pattern goes explicitly, in either of a builtin's spellings; anything ELSE
that pointed into the rig - a module pattern beneath it, an exact call inside it - is now
dangling and goes with the prune, which is the same sweep every other detection runs.

It returns the rows it saved, so the uninstall drops from the session's view any purview
the prune removed, as a purview deletion does. A failed load or save returns none and
leaves the view as it was.
