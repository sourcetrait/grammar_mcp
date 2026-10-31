# scenario.rs

## fn run_case
The case name and both manifests are checked before anything is written, so a
refused run leaves no result behind. A valid run of a case this build lacks
writes its result as `unrun` and exits 77: a case never passes by omission. An
implemented case writes `pass` with every assertion it ran, or `fail` with
those it ran through the one that failed and that failure as its reason.

## fn manifest
A manifest is NUON its model holds, each record exactly its declared fields,
and its `schema_version` this probe's. The candidate set's model holds its
version alone until the cases read more; the fixture's is the identity
manifest's, so a fixture run's redacted manifest is checked whole before any
case reads it.

## fn now_ms
Wall-clock milliseconds, for the record's timestamps alone; nothing waits on
them.
