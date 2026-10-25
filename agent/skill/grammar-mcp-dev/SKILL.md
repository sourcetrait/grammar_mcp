---
name: grammar-mcp-dev
description: Authoring, installing, and managing Grammar MCP equips and rigs
---
# Grammar MCP Development
An equip is a nu module tree you own, `equip/<author>/<name>/`, installed into
the MCP's signed repository: its `lib/` plain modules, its `rig/` the calls.
Using the tools is in grammar-mcp; the Nu language is in nu.

## Lifecycle
`source_dir` is the equip's `equip/<author>/<name>` directory; its `lib/` and
`rig/` install together, and a `cli/` stays behind.
- `equip(new, source_dir)`: register an empty equip, its `rig/mod.nu` seeded.
- `new([namepaths])`: scaffold modules or functions into an equip's rig.
- `equip(check, source_dir)`: validate without committing.
- `commit(namepath)`: validate and upsert; returns `{added, modified, removed}`.
- `equip(install, source_dir)`: validate and commit a finished source.
- `equip(uninstall, source_dir)`: drop the equip; its source is untouched.

An equip is `<author>/<name>`; a call `<author>/<name>:module/path:function`.
A builtin, `gram` or `grammar`, drops its author everywhere: `commit("gram")`.
Every call lives in a module of the rig, none at its root.

## Call target
A directory `<name>/` whose `mod.nu` exports one `main`:
```nu
# Doubles x.
export def main [args: record<x: int>]: nothing -> record<y: int> {
    { y: ($args.x * 2) }
}
```
- The `args` record type is the args schema; the output type the result's.
- Either may be `nothing` for void; a bare `record<>` is rejected.
- Its `mod.nu` may hold private helpers, never a submodule.
- `main` is reserved for this export alone.

## Wiring
Each directory's `mod.nu` in the rig wires its children:
- call target: `export module <name>` and `export use <name>`
- pure module, no `main`: `export module <name>` only
- flat helper file: `export use ./<name>.nu` or a private `use`; no `main`
```
rig/
  mod.nu             # export module shape
  shape/mod.nu       # export module area; export use area
  shape/area/mod.nu  # export def main ...
```
Wiring errors: `rig::call_wiring` (no `export use`), `rig::orphan` (unwired),
`rig::call_leaf` (a call holding a submodule).

## Imports
Always the full path, your own equip included, then drive by module name:
```nu
# rig/text/slug/mod.nu
use equip/acme/toolkit/lib/text
export def main [args: record<s: string>]: nothing -> record<out: string> {
    { out: (text normalize $args.s) }
}
```
- `equip/<author>/<name>/lib/<module>` is a lib's, `.../rig/<module>` a rig's.
- Import a sibling module directly, never your parent (`CircularImport`).
- Import a single function only to resolve a name collision.
- `./x.nu` is only for flat helper files.
- Another equip must be installed before yours can import it.

## Docs
The comment above `main` documents a call; a `mod.nu`'s leading comment, its
module, or the rig at the root.
Its first line is the summary (<= 80 chars); details follow a blank comment line.
Say only what names and types don't: units, side effects, edges, failures.

## Files
A rig carries only:
- `.nu` sources and `.gitignore` files
- root `rig.toml`, `README.md`, `LEGAL.md`, `LICENSE.txt`, `LICENSE-*.txt`
- root `.assets/`: data, no script extensions
- root `.docs/`: `.md` and `.txt`

A lib carries `.nu` files alone. Nothing may be executable; anything else is a
`rig::*_denied` or `lib::*_denied` error.
An equip can't be named `docs`, `tools`, `bin`, or `target`.

## Where equips live
- repository-local: `./equip/<author>/<name>/`
- shared: symlink `$XDGX_LIBRARY_HOME/nu/equip/<author>/<name>` to the source
