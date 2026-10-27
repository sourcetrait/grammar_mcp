# new.rs

## fn equip_scaffold
A BARE EQUIP NAMEPATH IS REJECTED with a message pointing at `interface equip new`. This
command scaffolds modules and functions into an established equip's rig; establishing an
equip is `interface equip new`'s job, so an author reaching for it that way needs the other
command named rather than a generic arity complaint.

THE BATCH IS ALL-OR-NOTHING ON PRE-EXISTENCE. Every target equip is locked in canonical sorted,
deduped order first - sorted because a consistent lock order is what prevents a deadlock
between two concurrent batches - then EVERY leaf is checked for pre-existence across the whole
batch BEFORE any is created. Otherwise a batch could half-apply and leave the author to work
out which half.

The scaffolding itself is not transactional beyond that check: a filesystem failure mid-batch
leaves what it already wrote.
