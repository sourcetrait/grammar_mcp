# The feature matrix's check: each profile's normal dependency closure, for
# each Linux target, held to its required and forbidden packages.
#
# The matrix is `build/nuon/daemon/feature_matrix.nuon`, its type
# `build/type/daemon/feature_matrix.nutype`. A check builds nothing: `cargo
# tree` and `cargo metadata` read the committed lock for any target, its
# standard library installed or not.

# The matrix a repository commits.
export def load [
    root: path
]: nothing -> record {
    open ($root | path join 'build' 'nuon' 'daemon' 'feature_matrix.nuon')
}

# Where a check's evidence lands by default: under the target directory cargo
# resolves in the repository.
export def evidence_dir [
    root: path
]: nothing -> path {
    let run: record = (cargo_run $root ['metadata' '--locked' '--no-deps' '--format-version' '1'])
    if $run.exit_code != 0 {
        error make {msg: $"cargo metadata failed: ($run.stderr | str trim)"}
    }
    $run.stdout | from json | get target_directory | path join 'grammard-evidence' 'feature-matrix'
}

# A profile's feature flags: each feature by its package, and the defaults off
# where the profile turns them off.
export def feature_flags [
    profile: record
]: nothing -> list<string> {
    if ($profile.features | is-not-empty) and (($profile.packages | length) != 1) {
        error make {msg: $"profile ($profile.name) names features across several packages"}
    }
    let features: list<string> = if ($profile.features | is-empty) {
        []
    } else {
        let package: string = ($profile.packages | first)
        ['--features' ($profile.features | each {|feature| $"($package)/($feature)" } | str join ',')]
    }
    let defaults: list<string> = if $profile.default_features { [] } else { ['--no-default-features'] }
    [...$features ...$defaults]
}

# A profile's `cargo tree` selection for a target: its packages, its feature
# flags, and the triple.
export def tree_args [
    profile: record
    triple: string
]: nothing -> list<string> {
    let packages: list<string> = ($profile.packages | each {|package| ['-p' $package] } | flatten)
    [...$packages ...(feature_flags $profile) '--target' $triple]
}

# A profile's normal closure for a target: the command, the tree as cargo
# prints it, a package a line, and the packages' names.
export def closure [
    root: path
    profile: record
    triple: string
]: nothing -> record<command: list<string>, tree: string, packages: list<string>> {
    let command: list<string> = [
        'tree' '--locked' '-e' 'normal' '--prefix' 'none' ...(tree_args $profile $triple)
    ]
    let run: record = (cargo_run $root $command)
    if $run.exit_code != 0 {
        error make {msg: $"cargo tree for ($profile.name) on ($triple) failed: ($run.stderr | str trim)"}
    }
    let packages: list<string> = (
        $run.stdout | lines | where {|line| $line != '' }
        | each {|line| $line | split row ' ' | first } | uniq | sort
    )
    {command: $command, tree: $run.stdout, packages: $packages}
}

# The workspace's metadata for a target under a profile's feature flags: the
# command and its JSON.
export def metadata [
    root: path
    profile: record
    triple: string
]: nothing -> record<command: list<string>, json: string> {
    let command: list<string> = [
        'metadata' '--locked' '--format-version' '1' '--filter-platform' $triple
        ...(feature_flags $profile)
    ]
    let run: record = (cargo_run $root $command)
    if $run.exit_code != 0 {
        error make {msg: $"cargo metadata for ($profile.name) on ($triple) failed: ($run.stderr | str trim)"}
    }
    {command: $command, json: $run.stdout}
}

# Why a package is in a profile's closure for a target: its inverted tree.
export def why [
    root: path
    profile: record
    triple: string
    package: string
]: nothing -> string {
    let run: record = (
        cargo_run $root ['tree' '--locked' '-e' 'normal' '-i' $package ...(tree_args $profile $triple)]
    )
    $run.stdout + $run.stderr
}

# Check every profile for every target: the trees, the metadata, and each
# forbidden package's inverted tree written under `out`, beside `matrix.nuon`,
# the verdicts with each file's sha256.
export def check [
    root: path
    out: path
]: nothing -> record<cargo: string, lock_sha256: string, ok: bool, checks: table<profile: string, triple: string, ok: bool, missing: list<string>, forbidden: list<string>, packages: int, tree_command: string, tree: string, tree_sha256: string, metadata_command: string, metadata: string, metadata_sha256: string>> {
    let matrix: record = (load $root)
    mkdir $out
    let checks: table = (
        $matrix.targets | each {|target|
            $matrix.profiles | each {|profile| check_profile $root $out $profile $target.triple }
        } | flatten
    )
    let verdict: record = {
        cargo: (^cargo --version | str trim)
        lock_sha256: (open --raw ($root | path join 'Cargo.lock') | hash sha256)
        ok: ($checks | all {|row| $row.ok })
        checks: $checks
    }
    $verdict | to nuon --indent 2 | save --force ($out | path join 'matrix.nuon')
    $verdict
}

# One profile on one target: its closure and metadata kept, held to its sets.
def check_profile [
    root: path
    out: path
    profile: record
    triple: string
]: nothing -> record {
    let stem: string = $"($profile.name).($triple)"
    let resolved: record = (closure $root $profile $triple)
    let tree_file: path = ($out | path join $"($stem).tree.txt")
    $resolved.tree | save --force $tree_file
    let meta: record = (metadata $root $profile $triple)
    let metadata_file: path = ($out | path join $"($stem).metadata.json")
    $meta.json | save --force $metadata_file
    let missing: list<string> = ($profile.required | where {|package| $package not-in $resolved.packages })
    let present: list<string> = ($profile.forbidden | where {|package| $package in $resolved.packages })
    for package in $present {
        why $root $profile $triple $package | save --force ($out | path join $"($stem).why.($package).txt")
    }
    {
        profile: $profile.name
        triple: $triple
        ok: (($missing | is-empty) and ($present | is-empty))
        missing: $missing
        forbidden: $present
        packages: ($resolved.packages | length)
        tree_command: (['cargo' ...$resolved.command] | str join ' ')
        tree: ($tree_file | path basename)
        tree_sha256: (open --raw $tree_file | hash sha256)
        metadata_command: (['cargo' ...$meta.command] | str join ' ')
        metadata: ($metadata_file | path basename)
        metadata_sha256: (open --raw $metadata_file | hash sha256)
    }
}

# Run cargo in `root`, its output and exit kept.
def cargo_run [
    root: path
    args: list<string>
]: nothing -> record<stdout: string, stderr: string, exit_code: int> {
    do { cd $root; ^cargo ...$args } | complete
}
