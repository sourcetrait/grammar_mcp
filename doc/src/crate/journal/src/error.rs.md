# error.rs

## enum JournalError
An open's two failures: the lock another process holds, which a later start
may find free, and a directory or a file the journal can't make, open, or
trust.

### fn into_io, fn from_io
A subsys system takes only data at its start and fails it with an I/O error, so
the open's error rides that error whole and the handle takes it back out by
its type, never by reading its text.

## enum StepError
An admitted operation's move that wasn't recorded: the journal degraded, or a
move the model doesn't have, an unknown operation's among them. Its caller
knows what the move came before, so the caller reports the outcome: nothing
ran when the engine wasn't yet asked, the outcome uncertain after.
