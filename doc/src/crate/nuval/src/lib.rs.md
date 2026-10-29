# lib.rs

## feature nu
nushell's own values and errors, converted to and from nuval's, are the
evaluator's side, and they take nu-protocol, miette, chrono, serde_json, and
snafu. The data takes none of them: the protocol and the client carry values
and errors without evaluating anything, so they build nuval without `nu`, and
the engine and the MCP crate build it with. Each conversion sits beside its
data behind `#[cfg(feature = "nu")]`; a kind's code and message, an error's
`short`, and every type's codec derives need no feature.

## mod kind
One file a nushell error type's family of kinds, the error model's tables.

## mod json
The serde_json bridge into nushell's values, behind `nu`, shared by the engine
and the MCP client.

## use snafu::OptionExt
A trait in scope for its `context` on an `Option`: a range's second value
overflowing, and a row index past the platform's usize.

## mod nu
nushell's items the conversions name, under one path, so a reader tells them
from nuval's own, behind `nu`. nushell's I/O error kind is `IoKind` here,
apart from nuval's `IoErrorKind`.

## use sourcetrait_common::cereal
cereal comes through the workspace's `sourcetrait_common`, its facade feature
set, rather than `sourcetrait_cereal` alone (User). `cereal::derived` emits
absolute `::serde`, `::bitcode`, and `::rkyv` paths, so those three are the
crate's own dependencies, at the instances cereal uses; miette and chrono are
taken at nu-protocol's.

## pub use
The value, its error and result, the data each kind of value holds, every
family of error kinds, and `wide`; with `nu`, the two conversion traits and
the conversion's error.
