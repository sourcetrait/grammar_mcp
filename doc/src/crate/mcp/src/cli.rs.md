# cli.rs

## struct Cli
One binary, its namespace and deny set selected at runtime by trusted operator
config. What it serves on is the build's: a bare invocation is the client in
every build, and the engine-backed test host is `test-driver`, under
`embedded-cli` alone (`run.rs`).

The validation here is deliberately uneven, and the unevenness is the design.
`id` and `namespace` are not ident-checked: they come from trusted operator
config (the `.mcp.json` entries), and a bad value surfaces as the natural
downstream error rather than being pre-empted here. `workdir` is expanded but
never existence-checked, on the same trust. `--deny` is validated, because a
typo silently denying nothing would defeat the operator's whole intent - the
failure mode differs in kind from the others, so the treatment does.

`namespace` is `Option<String>` rather than a clap `default_value`, and that is
load-bearing rather than stylistic: with a default, clap cannot distinguish an
explicitly-passed value from a defaulted one, and `--test` needs exactly that
distinction to supply a default without overriding an explicit choice.

The parse error in `parse_deniable` (`run.rs`) carries the full deniable list,
which is why the `--deny` help text does not repeat it. That is the general
shape of what the summary cap costs here: duplication, not information.

### field config
Named, the file must exist; unnamed, the host reads the agent's own file under
its start directory, whose absence is the ordinary zero-config state
(`config.rs`).

## enum CliCmd
`Cli` parses in every build, so a client build answers an operator's command
with the feature it lacks rather than clap's unknown-subcommand error.
`TestDriver` exists only under `embedded-cli` and is hidden from the help: it
serves the legacy test cohort, never an agent.

## enum CliTool
Deny does not apply to this surface. It gates agent registration, and the
one-shot path dispatches straight to the handler methods without the router, so
the operator always has the whole set.

The consumer of this output is a wrapper, never a human eye - a nu-native front
that takes real records and serializes at this argv boundary - which is why a
tool's data arrives as single-quoted NUON text, as an agent sends it, and the
output is the tool's NUON envelope as the wire carries it, with no tty branch
anywhere.

`Interface` takes the command as one string, the text an agent sends, so the
operator's command meets the same parse rather than a second grammar in clap.
