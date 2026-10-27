# rig.rs

The equip substrate is a full nu ECOSYSTEM, not merely a `call()` registry. A rig's call
targets are the indexed "bins", but EVERY export - a rig's call target or helper, a lib's
module - is importable from a body and from external nu scripting.

## const META_FILE
This file is ALSO the rig-detection marker: an installed equip whose `rig/` carries it HAS a
rig, and `LIB_META_FILE` marks its lib the same way. Detection keys on the exact filenames.

## const EQUIP_KINDS
The lib comes FIRST because the rig may import it: validation and the copy run in this order,
and a lib-only equip is as legitimate as a rig-only one. `cli` is absent by design - the MCP
never takes an equip's interactive kind.

## fn is_valid_equip
EXACTLY `<author>/<name>`: two parts, each a valid ident, neither `main`. A BARE equip with no
slash is REJECTED: there is one spelling of an equip everywhere, with no default-author
shorthand to disambiguate.

## fn equip_namepath_of
The namepath comes from the PATH, so the source's location is the one statement of the
equip's name: `.../equip/<author>/<name>` and nothing else. A `.` or `..` component is refused
rather than normalized, because the registered source is compared component-wise later and a
normalized alias would name a different directory than the one it spells.

## fn require_rig
Two refusals, kept apart: an equip that is not installed is `equip::not_registered`, while an
installed one with no rig is `equip::no_rig`. A call or a scaffold into a lib-only equip is a
misunderstanding of what was installed, not a typo in its name.

## struct RigLocks
The outer mutex guards only MAP MUTATION; the per-equip `RwLock` is what serializes work.
Call takes a READ lock, and commit / new / install / uninstall take WRITE, so different equips
are fully parallel on the read/validate side.

## static EQUIP_REPO_LOCK / fn equip_repo_lock
The repo-wide write lock the per-equip locks cannot supply. The belief that "git's index
locking serializes the subprocess commits" was WRONG and measured so: git fails a second
writer on an existing `.git/index.lock` immediately rather than waiting, so two concurrent
lifecycle ops on DIFFERENT rigs through one host spuriously failed (five of five parallel
integration sweeps, `index.lock` in each). One repo, one lock, held only across the
canonical-tree wipe/copy plus the git section - validation is the expensive half and stays
outside the hold, so different-equip validations still parallelize. The purview table rides
the same lock (`purview_add_rig`/`purview_remove_rig`): it is namespace meta beside the repo
whose load-mutate-save is not atomic. Namespace isolation per `(id, namespace)` still means no
cross-process race to guard; this lock is for one process's own concurrency.

### fn hydrate_from_disk
Pre-creates a lock per installed equip at startup, keying on either kind's meta file, so a
`lookup` during serving never has to decide whether a directory is an equip.

## fn ensure_keypair
`allowed_signers` is rewritten from the current pubkey on EVERY startup rather than only at
creation, so it cannot drift out of step with the key it authorizes.

## fn ensure_equip_repo
The empty init commit exists so HEAD is valid immediately - a fresh repo with no commit
makes the first `git status --porcelain` comparison meaningless.

`configure_repo` runs on every startup, not just creation, so a moved keypair path
re-applies rather than leaving a repo that silently cannot sign.

It reports whether it made the repository. A namespace whose repository this startup made
is NEW, and a new namespace is the one moment the host installs anything on a user's
behalf.

## fn configure_repo
PER-REPO configuration only. Nothing here leaks into the user's global git config, and
`commit.gpgSign` being set locally is what makes every lifecycle commit signed implicitly
rather than by remembering a flag.

## fn establish_equip
Writes an establish-time rig index recording `<source_dir>/rig` BEFORE any validation,
because that recorded path is what every later `commit` re-reads, by its parent, and what
`check_source_dir` cross-checks. The equip exists as an addressable empty rig first, and
gains content on its first commit.

## fn scaffold_leaf
EMITS THE CASCADE CONVENTION: a call gets BOTH `export module` and `export use`, while a
module-path segment gets `export module` alone. That is nushell's own prescription from the
0.114.0 release notes (PR #18303, "Submodules are no longer implicitly imported"), not a
house rule - importing a module no longer implicitly imports its exported submodules, and
the documented fix is to keep `export module` and ADD `export use`.

Verified against 0.114.1: with the inline `export module sub` alone, `use foo` then
`foo sub baz` fails; adding `export use sub` restores it.

The LEAF-GUARD refuses an existing terminal namepath, so scaffolding can never write over
authored work. Intermediate module dirs are additively wired and may already exist.

## fn read_summary
A summary is single-line in EFFECT but is hard-wrapped in SOURCE like any other comment, so
the stored text carries the author's line breaks and the wrap has to come back out.

THE SIGNATURE BLOCK STRUCTURALLY REQUIRES IT: an unflattened continuation lands at column
0, where the block's own grammar reads it as an AUTHOR line - so a multi-line summary would
break the block, a case a single-line test fixture never exercises.

Normalizing HERE rather than at each consumer is what keeps the block, the standalone
signature and inspect's `summary` fields agreeing. `details` is prose and keeps its
newlines, which is why it stays on `read_doc`.

## fn registered_equip_names
The plain sort of the compound `<author>/<name>` gives author-then-name ordering for free,
since the author is the leading segment. The repository's own `.git` is skipped by its dot,
and a directory carrying neither kind's meta is not an equip.

## fn registered_rig_names
The installed equips WITH a rig - the only ones that have calls, so the signature block and
the purview pruning read this set rather than every equip.

## fn with_summary
Shared by the block and the standalone form, so a summary is attached the same way wherever
a signature appears. The ` # ...` is omitted ENTIRELY when undocumented rather than left as
an empty comment.

## fn signature_of
THE TWO MODES OVER ONE IMPLEMENTATION. The TREE form passes the LEAF name, because the
indentation around it supplies the hierarchy; a STANDALONE inspect passes the FULL
NAMEPATH, because nothing around it does. Everything after that first token is identical,
which is the point - the two surfaces cannot drift.

A render failure logs and substitutes `<?>` rather than failing the whole block, so one bad
schema cannot make `info()` unusable.

## fn push_within_signatures
CALLS BEFORE SUBMODULES: the callables at a level are what a reader scans for, and a
submodule pushes the eye deeper. Each group sorts by name.

A module line is emitted when the module's OWN namepath is covered OR when anything below
it was. That second case is what keeps the ANCESTORS of a deep match present, so the
indentation still spells a whole namepath - a subtree without its ancestors cannot be turned
back into one, and the block's entire grammar is its indentation.

A PER-SIGNATURE PREDICATE rather than a rooted walk, because a purview is a SET of patterns:
two of them can root in different rigs at different depths, which no single root expresses,
while "does any of them cover this signature" composes for free. That predicate subsumes a
rooted walk - identical output for every single-pattern case - so one predicate covers it
rather than a predicate plus a separate rooted path.

The buffering is not stylistic: whether a module's own line belongs in the block is not
knowable until its subtree has been walked.

## fn render_signatures_within
THE ONE RENDERER. `info()` shows the current purview through it, a pattern `inspect()` shows
one pattern through it, and the whole namespace is just the `*` pattern - so none of the
three can drift. This is the agent's FIRST read after the skill, so it is compact by
design: a structured nested-JSON tree of the same surface runs roughly 8 KB for just two
rigs.

NOTHING IS STATED THAT CAN BE INFERRED, and the separators are inferable from shape alone:
depth 0 is an author and depth 1 its equip's rig, the two levels of `<author>/<name>`;
everything deeper is a module UNLESS it carries the two signature groups, which makes it a
call. So a reader joins author to name with `/`, module segments with `/`, and puts `:`
before the first module and before the call.

A BUILTIN takes depth 0 alone, under its bare name, and leads the block: its line already
reads as the namepath an agent calls, `gram:log:latest`, with no author to join. Every
other rig follows under its author as before.

NO TRAILING HIERARCHY CHARACTERS, and the reasoning generalizes. A trailing separator would
let a line state its own separator rather than have one inferred, but it fails at exactly
the MIXED MODULE, which no single character describes: `a/b:c/` plus `:call` needs an
override rule, and an override rule IS re-inference wearing a costume. Shape settles every
case including that one, because a call is recognizable on its own.

Two further arguments favour SHAPE over separators. REUSE: a new kind extends the format by
taking a new SHAPE - a helper would be `name [...]` beside a call's `name <> <>` - rather
than by carving another character out of a two-character vocabulary that is already
overloaded. Separators do not scale; shapes do. And this block is the PROTOTYPE for a
general nu-tree renderer, which argues for keeping it general rather than info()-shaped.

A rig appears when its OWN namepath is covered or when anything inside it is, which is what
lets a bare `author/` list a rig with no calls yet while a pattern naming a module the rig
does not have leaves no orphan heading behind.

PATTERNS MATCHING NOTHING RENDER EMPTY rather than erroring. A pattern is a filter, and an
unresolved `.` matches nothing BY DESIGN, so an empty block is already this format's answer
for "nothing here" - a fresh namespace renders empty for the same reason.

A rig whose index fails to decode SKIPS with a host-stderr note rather than failing the
call, so one corrupt rig cannot take the surface down.

## fn rig_signatures
One rig's slice of the block, walked from the depth its rig line sits above: a builtin's
one level shallower than an author's rig. The rig's read lock is held only while its index
is read and walked.

## struct CallDoc
There is NO `summary` field, and its absence is deliberate: SUMMARY IS PART OF THE
SIGNATURE, carried as the trailing ` # ...` exactly as the block does it.

## enum InspectDoc
UNTAGGED, so each member serializes as its own bare shape under the result's `doc`. The
caller always knows which member it will get, because it knows whether it passed a pattern
or an exact namepath.

The four shapes stay distinguishable by FIELD SET alone: `srcdir` marks a rig, `src` plus
`summary` a module, `src` plus `signature` a call, and a lone `signatures` a pattern.

## fn inspect_impl
Every path composes from the NAMEPATH against the COMMITTED canonical tree, so nothing here
reads `source_path`. Nothing agent-facing composes from it at all, so `source_path` is not
surfaced to the agent; it stays load-bearing INTERNALLY - `commit` re-reads it and
`check_source_dir` cross-checks it - so the index field stays.

A call's standalone signature names a builtin by its bare name, as the block does.

## fn cap_diagnostics
Caps each SEVERITY independently, so a flood of warnings cannot crowd out the errors that
actually block a commit.

## fn extract_doc
The first-blank-line split is nushell's OWN convention for a doc comment - the parser's
`build_desc` does the same thing to produce `description` and `extra_description` - so a
mod.nu leading comment and a `main` doc comment divide identically.

## fn check_summary_length
A WARNING rather than an error, so an over-long summary COMMITS and then surfaces in
`equip(check)`. Only Error-severity rows block.

## const NAME_DENYLIST
These names would collide with conventional directory meanings inside a namespace, so they
are refused wherever an equip is named rather than being allowed to produce a confusing
tree.

## struct SelfView
A temp `<tmp>/equip/<author>/<name>/<kind>` symlink per source kind, prepended to
`NU_LIB_DIRS`, so an equip's OWN equip-prefixed `use` - a rig importing its own lib or its
own modules - resolves against the source during its FIRST commit, before it has been placed
in the namespace at all. ONE view serves both kinds, which is what lets a rig's import of its
lib see the lib installed with it rather than a stale committed copy.

The symlink is needed because a call target's `main` parses DURING the rig's own load,
before the equip finishes registering: a bare in-tree self-reference would resolve to
nothing and surface as "External command failed", so the view gives its equip-prefixed
`use` something to resolve against.

The sequence number is what keeps two concurrent validations from sharing a view directory,
and the view exists before its first link, so a failed link drops it whole.

## fn validate_equip_source
Reports ALL findings across both kinds AND builds the rig's index and docs in the SAME walk,
so a commit never parses the tree twice. A finding's path carries `<equip>/<kind>/`, so it
names the kind it belongs to.

`ValidationResult::is_empty()` means "no ERROR-severity diagnostics" - errors gate a commit,
warnings advise.

## fn validate_lib_tree
A lib is plain nushell: no cascade, no call rules, no reserved term. Its one structural
rule is `.nu` alone, with no exemption for a `.gitignore` or a README, and nothing
executable.

## fn validate_lib_file
Each file parses as a module of its own, against the same engine and self-view a rig
gets, so a lib module importing a sibling or another equip resolves it as nushell would at
serve. A failed parse is checked for a cycle before its errors are reported, as in
`validate_flat_file`.

## fn validate_walk
A CALL IS AN EDGE MODULE: the walk validates it in place and never recurses into it, which
is what makes `rig::call_leaf` a rule rather than an accident.

Each child is tested against the edges NAMING it, and each check asks for the PRESENCE of a
required kind rather than keying on whichever edge was written first - a child may legally
carry more than one edge, which is exactly the case the convention prescribes.

THE VALIDATOR IS LOOSER THAN THE CONVENTION, deliberately for now: it requires an
`export use` edge on a call and SOME edge on a pure module, but does not require
`export module` on a call nor reject `export use` on a pure module. So a hand-authored call
wired `export use`-only still commits. The estate is now conformant, so tightening would
invalidate nothing; the ordering is the constraint - an enforcement change that turns valid
artifacts into violations cannot precede the sweep that makes them valid.

## fn parse_module_has_main
Classification by nushell's OWN `Module.main` sentinel rather than by grepping for
`export def main`, so the question "is this a call target" is answered by the parser that
will actually run it.

## fn extract_module_edges
A parse failure yields NO edges rather than an error, because the cascade validator reports
that parse error separately - returning edges from a broken parse would produce a second,
misleading orphan diagnostic on top of the real one.

## fn validate_flat_file
A FAILED parse is checked for a cycle BEFORE its errors are reported, as the eval path
does: a cycle's own parse errors are unstable in variant and payload and useless in every
form. A clean parse is never checked, since a diamond-shaped import graph registers its
shared modules many times without any cycle.

## fn scan_reserved_terms
`main` may appear ONLY as a call target's exported sentinel. The scan checks path components
first, then walks `flatten_block` FlatShape tokens.

Why those two shapes and no others: `VarDecl` catches let/mut/const names and `String`
catches def/module/alias names, record keys, cell-path members and barewords - EXCEPT a
`String` immediately after an `export def` InternalCall token, which is the valid sentinel
export. Quoted string VALUES keep their quotes in the token so they never match, and command
references parse as InternalCall or External so they skip. That leaves exactly
identifier-position `main` outside the sentinel.

A token's text is read through `span_text`, from the file's start in the span space,
never by the raw span: the engine's own files, the standard library's first, sit ahead of
this one, and a raw read returns nothing, which would pass `main` anywhere silently.

All three parse sites register the file's ABSOLUTE path as the parse fname AND push it onto
`working_set.files` before parsing. The PUSH is the load-bearing half: `path self` reads
`working_set.files.top()`, not the parse fname - the fname only feeds span mapping - so
without the push a parse-time `const SELF = (path self)` const-resolves at serve and fails
at validate. This mirrors nushell's own module-file loader.

## fn validate_mod_nu_ast
A resolved private `use` or `overlay use` parses to `Expr::ImportPattern` or `Expr::Overlay`
rather than a Call, so it is allowed BY SHAPE rather than by name - which is why those two
arms exist beside the decl-name allowlist.

`let` at module body level is a nu parse error and surfaces before this structural walk ever
runs.

## fn validate_function_file_ast
SCHEMAS ARE READ FROM SOURCE TEXT on both sides, and both parsed forms are lossy in
different ways. `SyntaxShape` Display renders an empty-field record as the bare word
`record`, which is not valid arg syntax, while the parsed output `Type` collapses path,
directory and glob to string. Structure is still VALIDATED against the parsed signature -
positional shape and output Type - so the text is used for the schema and the parse for the
contract.

That text is sliced from the wrapped source by the block's span less the file's start in
the span space (`file_start`), as every reported position here is measured. Measured from
zero, the slices come back empty once the engine holds files of its own - the standard
library's - and the call target drops from the index with no diagnostic.

## fn check_args_record_positional
An empty `record<>` is the UNFLESHED SKELETON and is rejected, which is what stops a
scaffolded stub from committing as if it were finished. `Type::Nothing` is the legitimate
void.

## fn registered_source
The equip's source is the PARENT of either kind's recorded source, the rig's first. A
registration is the per-kind meta alone, so there is no equip-level record to drift from the
kinds it describes.

## fn promote_equip
VALIDATES, never regenerates - author content and docs survive a commit untouched.

The equip's canonical subtree is WIPED before the copy, both kinds with it, so a kind gone
from the source leaves the namespace and each `.meta/` is rebuilt from this validation
rather than merged with the last one.

An empty porcelain means idempotent no-op: grouped-but-empty result and NO commit, so
re-committing an unchanged equip does not accumulate history.

The validator's WARNING rows ride in the result, the no-op's included: a source that
validates commits past its warnings, and the agent still reads them.

## fn install_equip
ATOMIC by order: the source is validated whole before anything is written, so a failed
install leaves nothing registered and nothing to roll back.

## fn copy_rig_tree
Mirrors the validator's carried set exactly. Root `.assets/` and `.docs/` copy VERBATIM
including dotfiles, while everything else skips dotfile entries at every level - which is
what keeps a client `.git` out of the signed repo.

`.gitignore` is honored only by commit's `git add` at the STAGING step, so it governs what
enters the repo and is NOT a validation exemption: the walk covers the full filesystem, so
an ignored-but-invalid file still blocks the commit.

## fn ensure_substrate
A NEW namespace gets the builtins; an existing one is never touched, so a builtin a user
uninstalled stays uninstalled, and nothing installs again on their behalf.

`ensure_default_purview` is non-fatal: purview bookkeeping must not stop the host from
serving.

`ensure_plugin_registry` is non-fatal for the same reason, and runs BEFORE the builtins,
since grammar's rig calls plugin commands its validation must know. It belongs to the
substrate because every engine base loads the registry it writes, so it must land before
the first base is built, on every entry point: the server, the one-shot CLI, and the test
server.

The locks hydrate LAST, so they hold the builtins installed a moment before.

## fn bundled_equip_dir
The deploy's own `equip/`, beside `bin/` as the plugins' `plugin/` is, so a deploy carries
the builtins' sources and the host finds them without being told where.

## fn install_builtins
Each builtin installs as `equip install` would, validated whole, its source the deploy's
copy. One the deploy doesn't hold is skipped, and a failure goes to stderr while the host
serves on: a builtin is a convenience, never a condition of serving.

The seeding runs once, for the builtins that installed: each one's own purview and its
reference in `core`. `default` keeps the `['*']` a new namespace starts with.
