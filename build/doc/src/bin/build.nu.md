# build.nu

The repository root's `build.nu` is a symlink to this script, so the script
never trusts the directory it was invoked from.

## The module imports
A relative `use` resolves against the directory of the path nushell was
handed, and through the root's symlink that is the root, where `../mod` does
not exist. `path self | path expand` follows the link to the real file, and
both it and `path dirname` evaluate at parse time, so the modules import
through constants built from it. Measured on 0.115.1 both ways: through the
link and directly.

## const ROOT
The same real path, three levels up, so the script builds the repository it
lives in from any working directory.

## def main
The signer resolves first, before any build, because a repository without a
usable key cannot produce a package and should say so in a second rather than
after the release builds. gram's equip is checked beside it for the same
reason.
