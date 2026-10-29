# Feature matrix

The daemon split's build profiles are `build/nuon/daemon/feature_matrix.nuon`,
typed and explained by `build/type/daemon/feature_matrix.nutype`. This note
says what each profile builds, how `grammar-mcp`'s entries differ between them,
how the closures are checked, and how each test cohort builds and runs.

## Profiles

| profile | packages | features | ships | what it builds |
|---|---|---|---|---|
| `client` | `sourcetrait_grammar_mcp` | none | `grammar-mcp` | the serving MCP, a client of its runtime |
| `runtime` | `sourcetrait_grammar_daemon` | none, defaults off | nothing | the runtime library, for grammard or an embedding host |
| `service` | `sourcetrait_grammar_daemon` | defaults (`service`) | `grammar-daemon` | grammard over the runtime library |
| `plugins` | `sourcetrait_grammar_eye`, `sourcetrait_grammar_soak` | none | `grammar-daemon` | the nushell plugins |
| `embedded_cli` | `sourcetrait_grammar_mcp` | `embedded-cli` | nothing | the MCP with its in-process engine and operator |
| `test_hooks` | `sourcetrait_grammar_mcp` | `embedded-cli`, `test-hooks` | nothing | the embedded CLI with the lifecycle tests' commands |
| `probe` | `sourcetrait_grammar_daemon_probe` | none | nothing | grammard-probe, the qualification driver |

The mcp crate declares no default features, so its shipped default and
`--no-default-features` resolve the same set. `test-hooks` names the engine's
feature weakly: alone it brings no engine, and a hook build selects
`embedded-cli` with it.

## grammar-mcp's entries

- The bare entry serves MCP over stdio as a client in every build and every
  feature combination, `--test` included. It never starts an engine and makes
  no engine or equip state. With no runtime endpoint, each runtime-owned tool
  and interface command answers `daemon::unavailable` in the existing
  envelope; the frontend-owned Channel commands, `open`, `ready`, and `close`,
  are the client's own. `runtime_ownership.tsv` names each operation's owner.
- `cli <tool>` runs one tool call on the in-process engine under
  `embedded-cli`. A build without it exits 2, naming the feature.
- `test-driver`, hidden, serves MCP over stdio on the in-process engine under
  `embedded-cli`, for the legacy engine-backed test cohort alone. A build
  without the feature has no such subcommand.
- `--test` selects the `test` namespace, and on the test driver the watchdog's
  sampling. It is never a backend.

## The closure check

`nu build/bin/matrix.nu [--out <dir>]` resolves every profile for every target
triple with `cargo tree --locked -e normal --prefix none`: the profile's
packages by `-p`, its features as `<package>/<feature>`,
`--no-default-features` where it turns the defaults off, and `--target` with
the triple. Every required package must be in the closure and no forbidden
one, however deep. Resolving builds nothing and needs no target standard
library.

Each profile and triple keeps its tree, the workspace's `cargo metadata
--locked --format-version 1 --filter-platform <triple>` under the same feature
flags, and each forbidden package's inverted tree. `matrix.nuon` holds the
verdicts with each file's sha256, cargo's version, and `Cargo.lock`'s sha256.
The evidence lands under `grammard-evidence/feature-matrix` in the target
directory cargo resolves unless `--out` names another; the exit is 1 when any
check fails.

## Each cohort's builds

The cohorts build one `grammar-mcp` binary under different features, so they
never share a target directory.

The client cohort builds in the target directory cargo resolves:

    cargo build --locked -p sourcetrait_grammar_mcp
    cargo test --locked --no-fail-fast -p sourcetrait_grammar_tests -- --test-threads=1
    cargo test --locked --no-fail-fast -p <each crate's own tests>
    cargo clippy --locked --workspace --all-targets

The embedded cohort builds in a target directory of its own, `--target-dir`
on each command, the plugins first:

    cargo build --locked -p sourcetrait_grammar_eye -p sourcetrait_grammar_soak
    cargo build --locked -p sourcetrait_grammar_mcp --features embedded-cli,test-hooks
    cargo test --locked --no-fail-fast -p sourcetrait_grammar_tests --features embedded-cli,test-hooks -- --test-threads=1
    cargo test --locked --no-fail-fast -p sourcetrait_grammar_mcp --features embedded-cli --lib --tests -- --test-threads=1
    cargo clippy --locked --workspace --all-targets --features sourcetrait_grammar_mcp/embedded-cli,sourcetrait_grammar_mcp/test-hooks,sourcetrait_grammar_tests/embedded-cli,sourcetrait_grammar_tests/test-hooks

The mcp crate's tests rebuild its binary without the hooks, so they follow the
system tests.

The system tests' harness spawns `test-driver` for a legacy host and the bare
entry for a client one. The tests crate's own `embedded-cli` and `test-hooks`
features select the cohort and pull in nothing: the client-serving tests and
the fixtures compile in every build, the legacy files under `embedded-cli`,
and the lifecycle tests under both. The mcp crate's integration targets, all
through its in-process `guts` facade, and its unit tests of the remote links
and the purviews compile under `embedded-cli` alone. `test_map.tsv` records
each test's profile and the entry it drives.

## The client build's lints

The mcp crate's root allows dead code and unused imports in a build without
`embedded-cli`: the client serves a narrow surface over the shared facade,
whose breadth serves the test driver. The `embedded-cli` build compiles the
whole crate with both lints and clippy runs over it, so dead code there still
fails.
