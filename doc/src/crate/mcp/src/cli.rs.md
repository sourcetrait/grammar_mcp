# cli.rs

## struct Cli
ONE BINARY, with the variant selected at runtime by trusted operator config -
which is why there is no build feature for a test host and no second target.

The validation here is deliberately UNEVEN, and the unevenness is the design.
`id` and `namespace` are NOT ident-checked: they come from trusted operator
config (the `.mcp.json` entries), and a bad value surfaces as the natural
downstream error rather than being pre-empted here. `workdir` is expanded but
never existence-checked, on the same trust. `--deny` IS validated, because a
typo silently denying nothing would defeat the operator's whole intent - the
failure mode differs in kind from the others, so the treatment does.

`namespace` is `Option<String>` rather than a clap `default_value`, and that is
load-bearing rather than stylistic: with a default, clap cannot distinguish an
explicitly-passed value from a defaulted one, and `--test` needs exactly that
distinction to supply a default without overriding an explicit choice.

The parse error in `parse_deniable` (`run.rs`) carries the full deniable list,
which is why the `--deny` help text does not repeat it. That is the general
shape of what the summary cap costs here: duplication, not information.

## enum CliTool
Deny does NOT apply to this surface. It gates agent REGISTRATION, and the
one-shot path dispatches straight to the handler methods without the router, so
the operator always has the whole set.

The consumer of this output is a WRAPPER, never a human eye - a nu-native front
that takes real records and serializes at this argv boundary - which is why
record-shaped inputs arrive as single-quoted NUON strings and the output is the
tool's NUON envelope as the wire carries it, with no tty branch anywhere.

`Interface` takes the command as one string, the text an agent sends, so the
operator's command meets the same parse rather than a second grammar in clap.
