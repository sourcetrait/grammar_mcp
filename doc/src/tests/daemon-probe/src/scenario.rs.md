# scenario.rs

## fn run_case
The case name and both manifests are checked before anything is written, so a
refused run leaves no result behind. A valid run of a case this build lacks
writes its result as `unrun` and exits 77: a case never passes by omission.

## fn manifest
A manifest is a JSON object whose `schema_version` is this probe's. The
fields the cases read join the check with the cases.

## fn now_ms
Wall-clock milliseconds, for the record's timestamps alone; nothing waits on
them.
