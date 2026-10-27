# config.rs

The record is the host's (`EngineHost::config_record`), mirroring the TOML; these
decls read it, walk it by a dotted key, and pin through the host.

## const KEY_SEP
The reason `get_config_all | get <key>` and `get_config <key>` agree: the key is a
PATH into the record the other decl returns, not a parallel naming scheme.

The walked form needs a LITERAL cell path, which is the trap here. A string
variable passed to `get` binds as one member containing the dots, so it looks for a
column of that whole name - splitting on dots is the source parser's job, not
`get`'s, so a `$key` string variable fails with
`Cannot find column 'supervisor.cpu_warn_fraction'`.

## fn config_keys
Derived from the record rather than restated beside it, so the vocabulary an error
message needs cannot drift from the vocabulary the record actually has.

## fn unknown_key
The title names the problem and the label the vocabulary, nushell's split.
`NuValError::short` carries both into the envelope's message, so the keys in the
title as well read twice. Listing the valid keys is the value of this error, and a
test asserts the list reaches the message once.

## fn value_type
Spelled as a union rather than `any`, because the set is CLOSED and naming it lets
a caller's own annotation be strict too.

## struct GrimmGetConfigAll

### fn signature
An OPEN record on purpose. The shape is documented by the `.nutype` model beside
the defaults, and restating it here would be a second place to keep in step.

## struct GrimmGetConfig

### fn run
An unknown key is an ERROR rather than a null, so a typo cannot read as
"configured to nothing". The two are indistinguishable at the call site and only
one of them is a bug the caller wants to hear about.

## struct GrimmPinConfig
Only the `[supervisor]` warning lines are pinnable. `[channel]` is excluded so
`interface channel config` stays the sole mutator there - a value with two mutators
is one whose effective setting depends on which surface you happen to ask.
Everything else in the config is not a runtime value at all.

A pin the host turns down is titled `grimm pin_config failed`, its reason the
label, as the sends' refusals are.
