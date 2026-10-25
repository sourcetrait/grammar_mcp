# The grammar-mcp release: cargo's builds, nushell, and the package's tree.

# The MCP's package: its binary is the release's, its version the release's.
const MCP_CRATE = 'sourcetrait_grammar_mcp'

# The file-name prefix of a plugin binary, the one the MCP registers by.
const PLUGIN_PREFIX = 'nu_plugin_'

# The skills the package carries, from `agent/skill/`.
const SKILLS = ['nu' 'grammar-mcp' 'grammar-mcp-dev']

# Build the MCP and its plugins in cargo's `release` or `debug` profile.
export def compile [root: path, profile: string]: nothing -> nothing {
    let flags: list<string> = (profile_flags $profile '--release')
    let crates: list<string> = ([$MCP_CRATE] | append (plugin_targets $root | get package | uniq))
    let packages: list<string> = ($crates | each {|crate| ['-p' $crate] } | flatten)
    ^cargo build --manifest-path ($root | path join 'Cargo.toml') ...$flags ...$packages
}

# Install nushell at the engine's version under `target/nu/<profile>`.
#
# Returns the installed `nu`. Cargo skips the build when that version is
# already installed there.
export def nushell [root: path, profile: string]: nothing -> path {
    let version: string = (engine_version $root)
    let nu_root: path = ($root | path join 'target' 'nu' $profile)
    let flags: list<string> = (if $profile == 'debug' { ['--debug'] } else { [] })
    (^cargo install nu --version $"=($version)" --locked --root $nu_root
        --target-dir ($root | path join 'target' 'nu' 'build') ...$flags)
    $nu_root | path join 'bin' 'nu'
}

# The engine's nushell version: `nu-protocol`'s in the workspace lock.
export def engine_version [root: path]: nothing -> string {
    (open --raw ($root | path join 'Cargo.lock')
        | from toml | get package | where name == 'nu-protocol' | get version.0)
}

# The release's semver, the MCP crate's version, `+dbg` for `debug`.
export def package_semver [root: path, profile: string]: nothing -> string {
    let version: string = (
        cargo_metadata $root | get packages | where name == $MCP_CRATE | get version.0
    )
    if $profile == 'debug' { $"($version)+dbg" } else { $version }
}

# Assemble the package's tree in `target/pkg/<profile>`; returns it.
export def layout [root: path, profile: string, nu_bin: path]: nothing -> path {
    let built: path = ($root | path join 'target' $profile)
    let tree: path = ($root | path join 'target' 'pkg' $profile)
    if ($tree | path exists) {
        rm --recursive $tree
    }
    mkdir ($tree | path join 'bin')
    mkdir ($tree | path join 'plugin')
    cp $nu_bin ($tree | path join 'bin' 'nu')
    cp ($built | path join 'grammar-mcp') ($tree | path join 'bin' 'grammar-mcp')
    for plugin in (plugin_bins $root) {
        cp ($built | path join $plugin) ($tree | path join 'plugin' $plugin)
    }
    for skill in $SKILLS {
        mkdir ($tree | path join 'skill' $skill)
        cp ($root | path join 'agent' 'skill' $skill 'SKILL.md') ($tree | path join 'skill' $skill)
    }
    ^chmod -R 'u=rwX,go=rX' $tree
    $tree
}

# cargo's flags for a profile: `flag` for `release`, none for `debug`.
def profile_flags [profile: string, flag: string]: nothing -> list<string> {
    match $profile {
        'release' => [$flag],
        'debug' => [],
        _ => { error make {msg: $"no cargo profile `($profile)`: release or debug"} },
    }
}

# The plugin binaries, by cargo's metadata.
def plugin_bins [root: path]: nothing -> list<string> {
    plugin_targets $root | get bin
}

# Each workspace binary named as a plugin, with its package, by cargo's metadata.
def plugin_targets [root: path]: nothing -> table<package: string, bin: string> {
    (cargo_metadata $root | get packages
        | each {|package|
            $package.targets
                | where {|target| ('bin' in $target.kind) and ($target.name | str starts-with $PLUGIN_PREFIX) }
                | each {|target| {package: $package.name, bin: $target.name} }
        }
        | flatten
        | sort-by bin)
}

# cargo's metadata for the workspace's own packages.
def cargo_metadata [root: path]: nothing -> record {
    (^cargo metadata --manifest-path ($root | path join 'Cargo.toml') --no-deps
        --format-version 1 | from json)
}
