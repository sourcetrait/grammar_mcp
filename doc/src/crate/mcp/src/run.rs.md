# run.rs

## fn set_cli_env_path
The flag's value goes to the UENV variable in front of its home as well: the
host reads a home's UENV variable first (`home.rs`), so a flag that set the
standard variable alone would lose to a UENV tree in the environment.

## fn set_cli_env
A `--xdg-*` value that does not expand is the operator's error: it comes back as
an `Err` and the host exits 2 before anything reads the environment.

## const WORKER_STACK_SIZE
Every runtime worker gets an eval thread's stack, in every build; under
`embedded-cli` an assertion holds it to the engine's `EVAL_STACK_SIZE`.

Nushell parsing is deeply recursive, and parses run inline on a runtime worker
rather than on an eval thread: the interface parse of an agent's command in
every build, and under `embedded-cli` an `execute` definition's contract check
with its lint and the equip validator. A pathological module graph, whose
resolution recurses until the accumulated path stops resolving, overflowed the
2 MB worker default, and so does a command nested 50 subexpressions deep on a
debug build. A stack overflow is a fatal runtime error rather than a panic, so
`catch_unwind` cannot save it and the host dies leaving no diagnostic at all.
That is what made it read as infrastructure rather than as code, and cost a
long diagnosis.

Sizing the runtime rather than each call site is deliberate - a per-site fix has
to be applied everywhere a parse can happen, and missing one leaves a latent
host death. The size bounds the depth rather than removing it: a module graph's
recursion has the filesystem's path limit for a ceiling, but a command's
nesting has none, and 1,500 levels overflow a debug build's worker as 20,000
overflow the released engine-backed host's.

## fn run
Constructed by hand rather than through `#[tokio::main]` for exactly one reason:
`thread_stack_size` (`WORKER_STACK_SIZE`).

The `tk::spawn` is not decoration either. `block_on` drives the future on the
thread that called it - `main`'s, whose stack `thread_stack_size` does not
govern - so without the spawn the serve path would get the sizing while the
one-shot CLI parsed on whatever the process was given. That gap survives in a
release build and core-dumps in a debug one, because unoptimized frames are
fatter, which is precisely the kind of difference that hides until it matters.

The runtime and the host task's join report and exit 1 rather than panic. The
join fails only when something under the task panicked, whose message is
already on stderr.

## fn serve_or_oneshot
The home directory is checked first, before the config reads it for the default
work dir, and its absence exits 2. This is `BASE_DIRS`'s first touch, after
`set_cli_env`'s overrides have landed (`home.rs`).

`CONFIG` is set before any dispatch, and before any reader, because `config()`
panics when read early - a startup-order bug rather than a runtime condition.

The bare entry serves as a client in every build (`server/client.rs`): no
engine starts on it, with or without `embedded-cli` and under `--test` alike,
so serving never makes engine or equip state. Only the local CLI and the test
driver start the engine, each behind `embedded-cli`.

Every end removes the scratch directories, whichever start made them: the
client's serve, the engine's under the CLI or the test driver, a failed start.
A signal's end removes them in its own sweep (`server/channel/face.rs`).

Exit code 2 is the operator-input code the one-shot CLI already uses, so a
rejected config and an unparseable argument report the same way. A serve that
ends in an error prints its reason after `grammar: ` and exits 1.

## fn run_cli
Under `embedded-cli` the engine starts first, its params from the config, mcp
its host, and nushell's config dirs, which every engine reads, resolved
(`server/host.rs`); a failure there exits 2, as a bad config does. Without the
feature the subcommand still parses, so an operator's command meets an answer
rather than clap's unknown-subcommand error: the refusal names the feature that
carries the CLI and exits 2, before anything is made.

## fn run_test_driver
The legacy engine-backed serving path, kept for the test cohort that needs an
engine behind the four tools: the engine's start as the CLI's, then
`run_server` (`server/run.rs`). It exists only under `embedded-cli`, its
subcommand hidden, so no shipped build serves on an engine.

## fn resolve_namespace
The namespace half of `--test` ends here, and that is what keeps the flag from
becoming a mode. From this function onward `test` is an ordinary namespace
string, so nothing downstream re-derives it, which leaves `Config.test` meaning
only the operator's sampling half.

## fn resolve_work_dir
Every path out of an argument expands through the same `expand_path` the config
file uses, so a `~` or a `$VAR` means the same thing whichever surface it
arrived on.

## fn resolve_config
The file is `--config`'s when one is named, else the agent's own under the
directory the host starts in, read once here: nothing an eval does moves the
host's own cwd, and nothing after the load reads the start directory again.
The remotes file's path is the config file's sibling, fixed here, though the
file itself is read at each remote command (`remotes.rs`).

The two surfaces are disjoint by design, so there is no precedence to resolve
between flag and file. What makes that safe rather than fragile is
`deny_unknown_fields` on the file layer: a file reaching for an argument is a
loud error instead of a silent no-op.
