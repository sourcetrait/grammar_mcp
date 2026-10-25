# bundle/mod.nu

What grammar-mcp's release holds, as against how an OptZip is packed.

## def compile
The MCP and the plugin crate build in one cargo invocation, so one lockfile
resolves both against the same nushell crates and the plugins speak the
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

## def plugin_bins
The plugin binaries come from cargo's metadata, so a plugin added to the
plugin crate reaches the package with no second list to keep.
