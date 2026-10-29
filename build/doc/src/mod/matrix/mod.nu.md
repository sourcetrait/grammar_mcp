# matrix/mod.nu

The feature matrix's check. Each profile's closure is resolved from the
committed lock for each target, never built, so a riscv64 profile is checked on
an x86_64 machine without its standard library, and the check stays seconds
long.

## def evidence_dir
The default lands under the target directory cargo resolves, read from `cargo
metadata`, never a `target/` the module assumes: the run's evidence is private
data, kept out of the source tree.

## def feature_flags
Each feature is written `<package>/<feature>`, the form cargo takes in a
virtual workspace's root, where `cargo metadata` runs; a profile naming
features across several packages is refused rather than guessed at, since a
bare feature would then apply to each.

## def closure
`-e normal` holds the profile to what ships in its binaries: a dev or build
dependency is no part of the closure the sets speak of. `--prefix none` leaves
one package a line, its name the first word, so a package anywhere in the
graph counts, however deep its edge.

## def metadata
The workspace's resolve under the profile's flags for the triple, kept beside
the tree as the proof of what the lock resolved to: exact versions and sources,
which the tree's names alone do not carry.

## def why
A forbidden package's inverted tree names the edge that brought it in, so a
failing check carries its own diagnosis.

## def check
Every profile on every target, the files written before their hashes are
taken, so `matrix.nuon` holds the sha256 of each file as it stands beside it,
with cargo's version and the lock's hash, the inputs the verdicts rest on.
