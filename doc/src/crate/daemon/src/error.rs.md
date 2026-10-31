# error.rs

## enum DaemonError
Plain data with a hand-written `Display`, so an embedding host matches on a
variant and grammard prints the same text it exits by. The instance's refusals,
a name, a home, a path, or the instance file, each carry what they refused, so
a report names the very name, home, path, or file at fault.

## fn DaemonErrorKind::name
The kind as a report or a test names it: the instance's under `instance::`,
scratch's under `scratch::`, and the start's own under `runtime::`.

## fn exit_code
sysexits' meanings, so a supervisor or a test tells the failures apart without
reading text: 69, `EX_UNAVAILABLE`, for a command this build lacks; 70,
`EX_SOFTWARE`, for a second runtime in one process, which is a caller's bug; 64,
`EX_USAGE`, for a name that is not one; 78, `EX_CONFIG`, for every home, path,
and instance file the start refuses; 73, `EX_CANTCREAT`, for scratch the worker
can't make or remove and a lock it can't take for a reason of the lock's own;
75, `EX_TEMPFAIL`, for a namespace another process holds, which a later start
may find free; and 70 for an engine that fails its start.
