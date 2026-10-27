# error.rs

## enum ConvertError
Each variant is a value nushell holds that nuval can't, or data nuval holds
that nushell wouldn't make.

### ErrorValue
A value that is itself an error carries the `ShellError` whole, so the
conversion of a result can take it as the result's error rather than refuse
it. A value nested anywhere in a list or a record counts: nushell raises such
an error when the value is used, so the whole result is that error.
