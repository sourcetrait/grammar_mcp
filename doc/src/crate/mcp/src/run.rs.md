# run.rs

## fn serve_or_oneshot
nushell's config dirs are resolved between the config and either surface, the
server and the one-shot CLI alike, because both build engines and every engine
reads them (`config_dirs.rs`). A failure there exits 2, as a bad config does.
