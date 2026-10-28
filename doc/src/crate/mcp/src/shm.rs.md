# shm.rs

A host's scratch directory: one private directory a process, made at its start,
read by every definition as `$env.EQUIP_SHM_DIR`, and holding the Channel's
inbox. The host makes it rather than reading one from the environment, so it
works on any machine whatever its environment's conventions, and a path no
other process shares needs no id inside it to keep two hosts apart.

## const SHM_BASE
Memory-backed where the host has it, since what lands here is transient bulk a
definition hands the agent, and a spill the agent reads once. A host without
`/dev/shm` falls back to the system's temporary directory.

## const SHM_PREFIX
The sweep's match: a directory not carrying it is never a scratch directory, so
the sweep cannot touch anything else in a shared base.

## const OWNER_FILE
The owner's pid and its start time, the identity the operator's pins use
(`process_start_time`): a pid alone is recycled, and the pair is not.

## fn make_shm_dir
`mkdtemp`, through `tempfile`, so the name is unguessable and the mode 0700 from
creation: no other user can enter it, and no other process can claim its name
first. The sweep runs before it, so a host's start is what cleans after a host
killed outright, which runs no exit hook. The marker is written after the
directory exists; a sweep meeting a directory between the two finds no marker
and leaves it.

Once a process: a second call returns the first directory, so a second start,
which the engine refuses, leaves no second directory behind.

## fn remove_shm_dir
Every ordinary end calls it, the server's, the one-shot's, a failed start's, and
a signal's (`run.rs`, `server/run.rs`). A directory already gone is no error.

## fn sweep_stale
Removes a directory only when its marker names a pair that is no longer a live
process. A directory without a readable marker is left alone: it may be a
host's still between its creation and its marker, or another user's, which the
mode keeps unreadable anyway.
