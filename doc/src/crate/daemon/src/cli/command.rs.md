# command.rs

## enum GrammardCmd
grammard's four namespaces: `serve` the user service, `instance` the
instances' registration and administration, `bridge` the controller's
subsystem bridge, and `cli` the local in-process CLI. Each is a bare command
for now, so a stray argument is clap's usage error and a known command
reaches its explicit refusal.

## fn name
The name a refusal prints, kept beside the variants so the two can't drift.
