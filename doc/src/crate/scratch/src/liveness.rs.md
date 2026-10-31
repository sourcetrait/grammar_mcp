# liveness.rs

## trait Probe
The process table as data, so a test answers what a host can't be made to: a
pid held by another user's process, a record that can't be read, a zombie, or
another boot.

## struct HostProbe
### fn process
The process table first (`sourcetrait_process`): a record gives the pid's
start. Where none is found the table can't say why, an exit or a refusal, so a
null signal asks: `ESRCH` means no process holds the pid, `EPERM` that another
user's does, and a success that one holds it whose record can't be read. The
signal sends nothing.

## enum Liveness
Three answers, since a sweep may act on a proof alone: an answer short of one is
unknown, never dead.

## fn liveness
Dead is proven four ways: an earlier boot; no process at the pid; a process at
the pid with another start, a later one reusing it, since two live processes
never share a pid; or a zombie, which has exited. A pid whose process can't be
read, another user's among them, is unknown, though a process of another user's
at the pid could not be the owner of a directory of this one's: the contract
counts a refusal as no proof.
