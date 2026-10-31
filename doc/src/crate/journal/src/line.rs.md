# line.rs

## const FORMAT
The header names it, so a reader knows an earlier segment's entries before it
reads them.

## struct Line
NUON, one record a line, where the Operation contract proposes bitcode frames:
every file the worker keeps is NUON. `seq` counts from the header's 0, so a
lost or a repeated line shows.

### fn new, fn read
The checksum is the SHA-256 of the line's `seq` and `entry` as nuval values,
encoded by bitcode: a form no text formatting moves, so a later writer's
spacing or quoting never breaks an earlier segment's sums. A line read back
holds only where its sum does.

## enum Entry
The segment's header, then the model's moves. An `Admitted` names the nonce the
ticket minted, the payload's hash, and the lease's epoch. Ids, nonces, and
hashes are text: a nonce's `u64` passes NUON's int.

## enum Stored
## struct Blob
A binary value in NUON, `0x[..]`, through serde's bytes, where a `Vec<u8>`
would write a list of ints.
