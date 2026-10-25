# Build grammar-mcp and its plugins, with nushell's own, into a signed OptZip.
#
# Usage: `nu build.nu [--debug]`, from anywhere.
#
# The package lands in `target/pkg/`,
# `grammar-mcp_<semver>_<os>_<arch>.opt.tar.zst`; its path is the last line.
# It is signed by the repository's gpg commit key, or by `gpg.signingkey`, a
# full fingerprint, with `gpg.homedir` the GnuPG home. gram's repository must
# sit beside this one, as `gram`: the package carries its equip.

const MOD_DIR = (path self | path expand | path dirname | path dirname | path join 'mod')
const BUNDLE = ($MOD_DIR | path join 'bundle')
const OPTZIP = ($MOD_DIR | path join 'optzip')
use $BUNDLE
use $OPTZIP

# The repository's root, from this script's real path.
const ROOT = (path self | path expand | path dirname | path dirname | path dirname)

# The program the package carries.
const NAME = 'grammar-mcp'

def main [
    --debug # cargo's dev profile; the package's semver takes `+dbg`
]: nothing -> nothing {
    let profile: string = if $debug { 'debug' } else { 'release' }
    let signer: record<key: string, home: oneof<nothing, string>> = (optzip signer $ROOT)
    bundle require_gram $ROOT
    let target: record<os: string, arch: string> = (optzip target)
    let semver: string = (bundle package_semver $ROOT $profile)
    let file: string = (optzip file_name $NAME $semver $target.os $target.arch)
    print $"building ($NAME) ($semver) for ($target.os) ($target.arch)"
    bundle compile $ROOT $profile
    print 'installing nushell and its core plugins'
    let nu_root: path = (bundle nushell $ROOT $profile)
    print 'laying out the package'
    let tree: path = (bundle layout $ROOT $profile $nu_root)
    print 'packing and signing'
    let out: path = (optzip pack $tree $NAME ($ROOT | path join 'target' 'pkg' $file) $signer)
    print $out
}
