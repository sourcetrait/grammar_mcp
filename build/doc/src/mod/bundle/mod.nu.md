# bundle/mod.nu

What grammar-mcp's release holds, as against how an OptZip is packed.

## def compile
The MCP and the plugin crates build in one cargo invocation, so one lockfile
resolves them all against the same nushell crates and the plugins speak the
engine's nu-plugin-protocol.

## def nushell
`cargo install --locked` builds nushell's own binary from crates.io against
nushell's published lockfile. Its version is the workspace's `nu-protocol`,
read from our lock, so the shell the package carries is the engine the MCP
embeds. A target dir under `target/nu/` keeps its build incremental, and cargo
skips the install outright when that version is already in place.

## def layout
The tree is emptied and rebuilt rather than updated, so a file dropped from
the release cannot linger in it. `chmod u=rwX,go=rX` leaves directories and
executables at 0755 and everything else at 0644, the modes the archive
records.

## def plugin_targets
A plugin is any workspace binary named `nu_plugin_*`, the prefix the MCP
registers a bundled plugin by, read from cargo's metadata, so a plugin crate
added under `crate/plugin/` reaches the package with no list to keep.
