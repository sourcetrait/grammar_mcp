# inventory.rs

## const GROUPS
The qualification contract's fourteen required groups, by the case names a run
takes. A run's case must be one of them, so a typo is refused rather than run
as nothing.

## fn inventory
Every group listed with `implemented` false and no assertions: a group's
assertions are enumerated by the gate that owns it, and an inventory that
claimed one before then would let a matrix count it. The capabilities are the
client's, the side the probe drives.
