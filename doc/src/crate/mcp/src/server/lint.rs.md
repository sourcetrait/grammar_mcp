# lint.rs

Two rule classes walk an agent's `nu` definition as a parsed AST. The lint exists to
TEACH passing values as data, so violations are agent-fixable rejects before any eval
dispatch or disk write.

## const ALLOWLIST_PATH_PREFIXES
Location-fixed contract paths, which pass uniformly because they cannot be passed as
data in any meaningful sense.

`/run/` is deliberately ABSENT - that one passes as `$XDG_RUNTIME_DIR` in data.

## const DENYLIST_EXTERNALS
Most of these have nushell builtins, so the fix is idiomatic nu; the rest pass in data
or go through a helper.

## const REGEX_RECEIVERS
The built-in signatures carrying a `--regex` flag, source-grep verified against
`nu-command/src/` at tag 0.113.1 and NOT re-audited since the 0.114.1 engine bump. A
decl that GAINED `--regex` in 0.114 would false-positive on its pattern argument, which
is carried as debt rather than assumed absent. Plugin commands are not covered at all.

## const PARSE_PATH_RECEIVERS
Decls whose positional 0 is a parse-time-const file or module path. Such a path CANNOT
pass in data - it resolves at parse time, before any data exists - so flagging it would
be a false positive by construction.

A RESOLVED `use` or `overlay use` parses to `Expr::ImportPattern` or `Expr::Overlay`
and is skipped by the walker's catch-all instead; this list covers the
module-not-found FALLBACK, which is a plain Call, plus `source` and `source-env`
always.

## fn lint_block
The walker over a block another parse produced, positions measured from that text's
start: execute.rs lints a `nu` definition on the parse its contract check already made,
the def call's body reached through its positional block like any closure's.

A diagnostic's position is measured from the text's start in the span space
(`file_start`): the engine holds the standard library's files ahead of this one, so a
raw span lands past the text. The rules themselves read the AST's values and never the
text. The lint reports RULES, not parse errors; the contract reports those.

THE FULL-SHELL ENGINE IS REQUIRED, not the lang-only one: lang-only collapses
`str replace --regex` into an ExternalCall where the flag is indistinguishable from a
positional, and the regex skip then cannot fire.

## fn push_diagnostic
`LINT_VIOLATION_CAP` bounds the list at three and the fourth candidate is dropped
SILENTLY. The `ControlFlow::Break` is threaded with `?` through every walker arm, which
is what short-circuits the recursion rather than merely discarding results. There is no
`more` marker on the wire - the envelope reports no internal-cap truncation.

## fn check_path
STRICT-UNIFORM by design: a bare `/` or a single-segment `/tmp` trips exactly like a
deep path, and there is NO glob-character exemption. The prefix rule is USE-BLIND - a
`str contains` filter marker and literal file content trip like a filesystem path -
because the alternative is inferring intent from a string, which cannot be done
reliably.

The `://` skip is what keeps a URL from reading as a path.

## fn check_external_head
Matches on the BASENAME, so `^/usr/bin/awk` matches `awk` AND separately trips the path
rule on the head literal. Both surface, which is correct: they are two different
violations that happen to share a token.

A variable-headed external (`^$data.cmd`) skips silently, because there is no literal
to judge.
