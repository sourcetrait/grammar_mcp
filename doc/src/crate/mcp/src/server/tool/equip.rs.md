# equip.rs

## enum EquipSummary
UNTAGGED: the caller knows the variant from the action it invoked, and the shapes are distinct.
`uninstall` returns no summary at all - void, matching its idempotent success.

## fn equip
The ACTION is checked before the source path, so an unknown action is named as one whatever
path rode with it. The namepath then comes from the path alone: the equip is wherever its
source says it is.

`new` and `install` REGISTER THE LOCK FIRST and unregister on failure. That ordering is what
makes "already registered" a lock-level answer rather than a filesystem race, and it is why
both error paths call `unregister` explicitly.

`uninstall` on an unknown equip returns success with no summary rather than an error,
because absent IS the requested state.

The guard is dropped explicitly before `unregister` in the uninstall path: unregistering
while still holding the lock would drop the map entry the guard belongs to.

An install brings a RIG into view only when the equip has one; a lib alone has no calls for
a purview to show.

## fn purview_add_rig
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
