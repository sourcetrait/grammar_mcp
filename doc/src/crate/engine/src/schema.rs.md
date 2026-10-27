# schema.rs

The contract's types: one `NuTypedef` AST, read from an `execute` definition's or a rig
`main`'s parsed signature, and the strict check that holds a value to it. A typedef is
DATA, serializable whole, so the rig index records each call's data and result typedefs
as they are and a call is held to them without its rig's source being parsed again.

The contract is nushell's own, User's ruling: an agent writes Nu signatures and NUON
values, so the contract has no second schema language, an agent's data crosses as NUON
with no JSON conversion, and there is no void. `nothing` is a type like any other, at the
top as below it.

`FieldName` and `ColumnName` are shared newtypes, since a NAME carries no representation
difference, but they stay distinct FROM EACH OTHER because a field and a column are not
interchangeable at any call site.

NO FALLBACK ANYWHERE: a type is the grammar's or it is refused, by name.

## enum NuScalarTypedef
`path` and `directory` stand as scalars of their own though nushell's parsed `Type` folds
both into `string`: a `data` type is read from its `SyntaxShape`, which keeps them, so a
signature shows what its author declared.

## fn render_signature_typedef
THE SIGNATURE RENDERERS are a PARALLEL pair rather than a flag threaded through the full
renderer, so each stays readable on its own. They share the `NuTypedef` AST, so the two
can only ever differ in PRESENTATION, never in what they understand a type to be. The
separators lose their spaces throughout.

## fn signature_group
The one difference from the full spelling is at the top: a record collapses to its
fields, `<refs:list<string>>`, the common case, while a table or `nothing` is spelled
whole, `<table<...>>` or `<nothing>`. Nested types keep their full spelling, so an open
record reads `record<>` inside one.

Every call has a data and a result typedef, so every call line carries its two groups:
that is what makes a call self-identifying in the signature block, which states no
separator characters at all.

## const OPEN_AT_TOP
An open record or table at the top of `data` would take anything, so a call's data would
document nothing. It is also the scaffold's skeleton, `data: record<>`, so the same
refusal is what stops a stub from committing as if it were finished.

## fn data_typedef / fn shape_typedef
The `data` type, read from its parsed `SyntaxShape`. At the top only `nothing`, or a
record or table with fields; below it the whole grammar, an OPEN record or table among
it, empty fields being nushell's own spelling of open. `any`, closures, blocks, and the
parser's other shapes are no data types and are refused by name.

## fn result_typedef / fn type_typedef
The return type, read from the signature's output `Type`. The same top rule, and an open
record or table refused at EVERY depth: a return declares what the body produces, and an
open type there would assert nothing.

## fn list_typedef
A list of tables is refused in both readings. nushell treats `table<C>` and
`list<record<C>>` as one type, so `list<list<record<...>>>` is a list of tables too and is
caught by its inner list of records. The refusal names the fix, a record holding each
table.

## fn check_value / fn check_fields / fn scalar_conforms
The strict check NuNu's contract asks for, which nushell's own makes no attempt at: its
`compare_types` and `is_assignable_to` hold a record OPEN, taking one with an extra field
as a subtype, so an undeclared field passes at any depth, and it checks no output when a
command runs at all. This walk holds a value to its declared type itself: each record
carries exactly its declared fields, each declared one required and an undeclared one
refused, each table row likewise, a list's every item, and a oneof's first member that
conforms. An EMPTY field list is an open record or table, and the walk stops there: that
is the one place any field stands.

A scalar is read by the value's own VARIANT, so `int` refuses a float, `float` refuses an
int, and `number` takes either, as nushell's types do. `path` and `directory` values are
strings by then, and a `glob` read back from NUON is a string too, so each takes one.

Every refusal names its PATH from the root the caller gives, `data` or `result`, with a
`.field` per record and an `[index]` per row or item: what an agent needs to find the one
wrong value in a large table.

## fn typedef_text
`render_nu_typedef` by another name, for messages and the test harness: an open record
renders `record<>`, where nushell's own Display would print a bare `record`, which is no
valid type syntax.
