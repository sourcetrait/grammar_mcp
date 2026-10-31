# owner.rs

## const MARKER_FILE
NUON, held to `include/scratch/Owner.nutype`, as every file the worker keeps
is.

## const LEGACY_MARKER_FILE
The MCP's marker before the guard, `<pid> <start>` on one line, which the MCP
still writes. It names no UID, so the directory's owner stands for it, and no
boot.

## const MARKER_MAX
A marker is a few dozen bytes; the bound keeps a sweep's read of a file in a
shared base small whatever the file holds.

## struct Owner
A pid alone is recycled; with its start it names one process within a boot.
Linux counts a start in clock ticks since the boot, so a pair from an earlier
boot can name a process of this one, and its boot id keeps them apart; macOS
counts from the epoch, so its start alone does, and its boot is none.

### field nonce
Each directory's own, so a guard's removal can tell its directory from another
made at the same path: it removes only a directory whose marker still holds its
nonce.

## fn current
The nonce's sixteen bytes come from `/dev/urandom`, which every platform the
guard builds on keeps.

## fn write
`create_new` and no symlink followed, so the marker is a file of this process's
making inside the directory it just made.

## fn read
A predecessor's marker is read only where the guard's is absent, and the two
are held to the same checks after.

## fn checked
A pid of 0 names the caller's process group to a signal, and a negative one
every process it may signal: a marker naming either names no one process, and
its pid never reaches a signal.

## fn current_boot
`/proc/sys/kernel/random/boot_id`, which the kernel draws anew at each boot.

## fn read_marker
Opened without following a symlink and without blocking, then checked through
that handle as a regular file, so a FIFO or a device in a directory is refused
rather than waited on, and nothing can be swapped in between the check and the
read.
