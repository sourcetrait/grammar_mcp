# lock.rs

## const LOCK_FILE
The MCP host's own lock in the namespace's data home: two evaluators on one
namespace would write one equip store at once. A worker refuses the namespace
while anyone holds the lock, a supervised MCP host among them. The MCP's
embedded entries don't yet take it before their engine, its one-shot CLI taking
none and its supervised host warning past a held one, so a worker excludes
another worker alone until EmbeddedCli brings them to the lock.

## struct LockRecord
NUON held to `include/runtime/grammard/Lock.nutype`: the worker by its identity
and by its process, as a scratch marker names one, and its generation for this
start. A contention report shows it as its holder wrote it.

## struct NamespaceLock
### fn acquire
`flock`'s exclusive lock, taken without waiting: a second start fails at once
rather than queueing behind the first, and the kernel releases the lock with
its holder's last descriptor, so a killed worker's lock passes to its restart
with nothing to clean. The file is opened without following a symlink and
without blocking, then checked as a regular file the UID owns. It is never
removed or replaced.

### fn record
Written in place over the last holder's record, the file truncated rather than
replaced: a rename would leave the holder locking an inode no later start
opens, and a second start would take the new one beside it.

## fn holder_of
The holder's record read through the refused descriptor, flattened to one line
and bounded, so the refusal names who holds the namespace.
