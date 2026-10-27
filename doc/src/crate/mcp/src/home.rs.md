# home.rs

The namespace's homes. The engine takes the two as its params, its data home and
its cache home (`server/host.rs`).

## static BASE_DIRS
`directories::BaseDirs` honors `$XDG_CACHE_HOME` and `$XDG_DATA_HOME`, which is
precisely what lets every in-process test isolate its state under a per-binary
tempdir - the harness sets those variables before the first access here.

It is `None` where no home directory can be found. `serve_or_oneshot` runs
`check_base_dirs` before anything reads it and exits 2 without one, so
`base_dirs`, the one reader, finds it set.

## fn cache_base_dir
ID FIRST, then namespace, so an agent's whole state across every namespace is ONE
subtree to wipe or back up. The vendor and app segments above that keep the tree
from colliding with anything else under the XDG root.
