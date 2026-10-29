# error.rs

## enum DaemonError
Plain data with a hand-written `Display`, so an embedding host matches on a
variant and grammard prints the same text it exits by.

## fn exit_code
sysexits' meanings, so a supervisor or a test tells the failures apart without
reading text: 69, `EX_UNAVAILABLE`, for a command this build lacks, and 70,
`EX_SOFTWARE`, for a second runtime in one process, which is a caller's bug.
