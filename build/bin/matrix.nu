# Check the feature matrix: each profile's normal dependency closure, for each
# Linux target, against its required and forbidden packages.
#
# Usage: `nu matrix.nu [--out <dir>]`, from anywhere.
#
# The trees, the metadata, and `matrix.nuon`, the verdicts with each file's
# sha256, land in `--out`, by default `grammard-evidence/feature-matrix` under
# the target directory cargo resolves. The verdicts print, then the directory;
# the exit is 1 when any profile fails on any target.

const MOD_DIR = (path self | path expand | path dirname | path dirname | path join 'mod')
const MATRIX = ($MOD_DIR | path join 'matrix')
use $MATRIX

# The repository's root, from this script's real path.
const ROOT = (path self | path expand | path dirname | path dirname | path dirname)

def main [
    --out: path # where the evidence lands
]: nothing -> nothing {
    let dir: path = if $out == null { matrix evidence_dir $ROOT } else { $out | path expand }
    let verdict: record = (matrix check $ROOT $dir)
    let rows: table = ($verdict.checks | each {|row|
        {
            profile: $row.profile
            triple: $row.triple
            ok: $row.ok
            packages: $row.packages
            missing: ($row.missing | str join ' ')
            forbidden: ($row.forbidden | str join ' ')
        }
    })
    print ($rows | table)
    print $dir
    if not $verdict.ok {
        exit 1
    }
}
