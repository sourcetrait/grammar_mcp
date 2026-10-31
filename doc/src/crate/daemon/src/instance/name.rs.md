# name.rs

## struct InstanceName
The runtime contract's form for an id and a namespace alike. A letter or a digit
first rules out `.`, `..`, and every hidden or option-like name, and the rest
holds no separator, NUL, space, or byte past ASCII, so a name is always one
plain path component. A name outside the form is refused with the field it was
given as; nothing is sanitized, since a sanitized name could become another
instance's.
