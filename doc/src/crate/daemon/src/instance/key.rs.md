# key.rs

## struct InstanceKey
The contract's key: each part's byte length as a big-endian `u32`, then its
bytes, the id's and then the namespace's, hashed with SHA-256 and written in
lower-case hex. The length prefixes keep `ab`/`c` and `a`/`bc` apart, which a
plain concatenation would not. The UID is no part of it: the key names an
instance's files within one account's homes, and the account is the path and
ownership boundary of its own.
