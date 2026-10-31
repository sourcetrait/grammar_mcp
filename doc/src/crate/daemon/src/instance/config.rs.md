# config.rs

## const INSTANCE_MODEL
The instance file's schema, `include/config/grammard/Instance.nutype`, compiled
in, so a file is held to the shape this build reads and not to one installed
beside it.

## const SUN_PATH_MAX
`sockaddr_un`'s path capacity, its terminator among its bytes: 108 on Linux,
104 on macOS and the BSDs.

## struct SupervisorLines
The MCP's four `supervisor.*` lines, by the same names.

## struct InstanceFile
The file as written. Its id, namespace, and key say which instance it is for,
and a load holds all three to the instance it was asked for, so one file can
never serve another. Its workdir is required; the peer links' config, the
supervisor's lines, and the limits are optional, null leaving the built-in
default.

## struct InstancePaths
### fn of
Every path is a function of the identity and the homes alone: the instance file
under the config home, durable state under the state home by key, the engine's
data and cache homes under the MCP's own `grammar/mcp/<id>/<namespace>` layout,
so a worker and an embedded engine share one namespace's files, and the socket
under the runtime home. A socket's path is measured with its terminator against
`sockaddr_un` and refused past it rather than truncated, since a truncated path
would bind another file.

## struct InstanceConfig
### fn load
Everything a start needs before any thread, listener, plugin, or engine: the
names checked, the key computed, the homes resolved, the paths derived, the file
read and held to its identity, and the workdir found an absolute directory. A
loaded config is a value: a later edit of the file changes nothing it holds.

## fn read_instance_file
The file is opened without following a symlink and without blocking, then
checked through that handle: nothing can be swapped in between the check and
the read, and a FIFO is refused rather than waited on for a writer. A file
another UID owns, one open to its group or to others, and anything not a
regular file are refused before a byte is read; the text must be NUON, hold the
schema's shape, and deserialize, each failure the file's own error.
