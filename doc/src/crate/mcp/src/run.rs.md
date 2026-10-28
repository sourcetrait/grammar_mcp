# run.rs

## fn set_cli_env
A `--xdg-*` value that does not expand is the operator's error: it comes back as
an `Err` and the host exits 2 before anything reads the environment.

## fn run
Constructed by hand rather than through `#[tokio::main]` for exactly one reason:
`thread_stack_size`.

Nushell parsing is deeply recursive, and two parses run inline on a runtime
worker rather than on an eval thread - an `execute` definition's contract check
with its lint, and the equip validator. A pathological module graph, whose
resolution recurses until the accumulated path stops resolving, overflows the
2 MB worker default and aborts the process. A stack overflow is a fatal runtime
error rather than a panic, so `catch_unwind` cannot save it and the host dies
leaving no diagnostic at all. That is what made it read as infrastructure
rather than as code, and cost a long diagnosis.

Sizing every worker like the eval thread removes the class rather than one
instance: the recursion is bounded by the filesystem's path limit, so its depth
has a finite ceiling whatever module graph reaches it. Doing it at the runtime
rather than per call site is deliberate - a per-site fix has to be applied
everywhere a parse can happen, and missing one leaves a latent host death.

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

`CONFIG` is set before serve or one-shot dispatch, and before any reader,
because `config()` panics when read early - a startup-order bug rather than a
runtime condition.

The engine starts between the config and either surface, the server and the
one-shot CLI alike, because both build engines: its params from the config, mcp
its host, and nushell's config dirs, which every engine reads, resolved
(`server/host.rs`). A failure there exits 2, as a bad config does.

The engine's start makes the scratch directory (`shm.rs`), so every end after
it removes the directory again: a failed start, the server's end and the
one-shot's alike. A signal's end removes it in its own sweep
(`server/run.rs`).

Exit code 2 is the operator-input code the one-shot CLI already uses, so a
rejected config and an unparseable argument report the same way. A serve that
ends in an error prints its reason after `grammar: ` and exits 1.

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

The two surfaces are disjoint by design, so there is no precedence to resolve
between flag and file. What makes that safe rather than fragile is
`deny_unknown_fields` on the file layer: a file reaching for an argument is a
loud error instead of a silent no-op.
