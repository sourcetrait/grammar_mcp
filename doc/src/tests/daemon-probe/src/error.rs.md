# error.rs

## fn exit_code
The statuses the qualification matrix reads: 0 only when every requested
assertion ran and passed; 1 a case that ran and failed an assertion; 77 a case
this build or its environment can't run, which the matrix never counts as a
pass; 78, `EX_CONFIG`, an unknown case, a malformed manifest, or an output that
already exists; 74, `EX_IOERR`, an output that couldn't be written.
