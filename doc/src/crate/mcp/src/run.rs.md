# run.rs

## fn set_cli_env
A `--xdg-*` value that does not expand is the operator's error: it comes back as an
`Err` and the host exits 2 before anything reads the environment.

## fn run
The runtime and the host task's join report and exit 1 rather than panic. The join
fails only when something under the task panicked, whose message is already on
stderr.

## fn serve_or_oneshot
The home directory is checked first, before the config reads it for the default
work dir, and its absence exits 2. This is `BASE_DIRS`'s first touch, after
`set_cli_env`'s overrides have landed (`server/cache.rs`).

nushell's config dirs are resolved between the config and either surface, the
server and the one-shot CLI alike, because both build engines and every engine
reads them (`config_dirs.rs`). A failure there exits 2, as a bad config does.

A serve that ends in an error prints its reason after `grammar: ` and exits 1.
