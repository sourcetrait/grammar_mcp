# bundle/mod.nu

What grammar-mcp's release holds, as against how an OptZip is packed.

## const CORE_PLUGINS
nushell's core plugins are pulled from crates.io as published, never
mirrored into the workspace: upstream's own `main` and its own lockfile
build each one, so the list of names is the one thing kept here.

## const EQUIPS
The builtins the package carries for the host to install into a new namespace:
grammar's equip from this repository, gram's from its own repository through
`ext/gram`, a relative link to a clone beside this one, so the build names no
path outside the tree.

## def require_gram
gram's equip is the one source outside the tree, so it is checked first, with
the signer, and a missing clone stops the build in a second, naming the
requirement, rather than after the release compiles.

## def compile
The MCP and the workspace's plugin crates build in one cargo invocation, so
one lockfile resolves them all against the same nushell crates and the
plugins speak the engine's nu-plugin-protocol.

## def nushell
`cargo install --locked` builds nushell's own binary and its core plugins
from crates.io, each against its published lockfile. Their version is the
workspace's `nu-protocol`, read from our lock, so the shell the package
carries is the engine the MCP embeds and every core plugin speaks its
protocol. A target dir under `target/nu/` keeps their builds incremental,
and cargo skips a crate outright when that version is already installed.

## def layout
The tree is emptied and rebuilt rather than updated, so a file dropped from
the release cannot linger in it. `plugin/` keeps its plugins apart by kind:
`core/` nushell's, `grammar/` the workspace's own; `rig/`, for custom
plugins, is the MCP's to read when one exists. `equip/` holds each builtin's
equip as its source holds it, the host's install taking its `lib/` and
`rig/`. `chmod u=rwX,go=rX` leaves directories and executables at 0755 and
everything else at 0644, the modes the archive records.

## def plugin_targets
A plugin is any workspace binary named `nu_plugin_*`, the prefix the MCP
registers a bundled plugin by, read from cargo's metadata, so a plugin crate
added under `crate/plugin/` reaches `plugin/grammar/` with no list to keep.
