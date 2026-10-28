# scratch.rs

A host's scratch directories, two a process, each private to it, made at its
start, and removed at its end: one for bulk, read by every definition as
`$env.EQUIP_TMP` and holding the Channel's inbox, and one for small runtime
files, `$env.EQUIP_TRANSIENT`. The host makes them rather than reading
directories from the environment, so it works on any machine whatever its
environment's conventions, and a path no other process shares needs no id
inside it to keep two hosts apart. The split follows the XDG Base Directory
spec, which keeps larger files out of `XDG_RUNTIME_DIR`, since it may sit in
memory that cannot be swapped out.

## const TMP_DEFAULT
Where the temporary directory sits when `TMPDIR` names none, as Rust's own
`temp_dir` has it.

## const SCRATCH_PREFIX
The sweep's match: a directory not carrying it is never a scratch directory, so
the sweep cannot touch anything else in a shared base.

## const OWNER_FILE
The owner's pid and its start time, the identity the operator's pins use
(`process_start_time`): a pid alone is recycled, and the pair is not.

## const SCRATCH_MODE
Asked of `tempfile` outright: its directories default to 0777 less the umask,
which left one 0755 under a 022 umask. The umask can only narrow the mode asked
for, never widen it.

## struct ScratchDirs
The bulk directory beside the transient one. Both always exist, so a definition
reads either without asking which the host could make.

## fn make_scratch_dirs
The bulk directory goes in the temporary directory, `TMPDIR` or `/tmp`; the
transient one in `XDG_RUNTIME_DIR`, or, where that names no directory, beside
the bulk one with a warning, the fallback the XDG spec asks for. Each base is
swept first, so a host's start is what cleans after a host killed outright,
which runs no exit hook; a base the two share is swept once. A base that does
not exist fails the start: a platform's directory is checked, never made.

A transient directory that cannot be made takes the bulk one with it, so a
failed start leaves neither.

Once a process: a second call returns the first directories, so a second start,
which the engine refuses, leaves none behind.

## fn tmp_dir / fn transient_dir
`None` before the start, which a caller reports as the host having no scratch
directory.

## fn remove_scratch_dirs
Every ordinary end calls it, the server's, the one-shot's, a failed start's, and
a signal's (`run.rs`, `server/run.rs`). A directory already gone is no error.

## fn env_dir
The XDG spec has a relative path in its variables ignored, and `TMPDIR` is held
to the same. An empty value reads as unset, and a relative one is reported as
ignored, so a misconfiguration shows rather than quietly moving the files.

## fn make_dir
`tempfile`'s builder, as `mkdtemp` makes one: the name unguessable and the mode
`SCRATCH_MODE` from creation, so no other user can enter it, and no other
process can claim its name first. The marker is written after the directory
exists; a sweep meeting a directory between the two finds no marker and leaves
it.

## fn sweep_stale
Removes a directory only when its marker names a pair that is no longer a live
process. A directory without a readable marker is left alone: it may be a
host's still between its creation and its marker, or another user's, which the
mode keeps unreadable anyway.
