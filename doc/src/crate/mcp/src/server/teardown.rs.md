# teardown.rs

The host's half of an eval's teardown. The engine kills an eval's external tree by a /proc
descendant walk (the engine's `teardown.rs`); the walk reaches a grandchild only while
that grandchild stays on the host's ppid chain, which is what the subreaper here keeps it
on, and adoption leaves zombies the host must then reap.

## const REAP_GRACE_SECS
THE GRACE IS THE SAFETY MECHANISM, not a tuning knob.

A blanket `waitpid(-1, WNOHANG)` would be the obvious implementation and is WRONG here:
it races the legitimate waiters - `Command::output()` inside `run_git`, and nushell's own
external handling - and can steal a child's exit status, so their wait fails ECHILD and a
working `run_git` returns a spurious error.

Those waiters reap within MILLISECONDS of the child exiting, so a zombie still sitting
there seconds later provably belongs to nobody. Age is what separates "abandoned" from
"someone is mid-wait", without needing a registry of every pid the process is currently
awaiting.

## fn parse_state_ppid
Fields are counted after the LAST ')' because the comm field is unquoted and can itself
contain spaces and parens. After it, index 0 is state (field 3) and index 1 is ppid.

## fn parse_start_time
Exists for the PID-REUSE GUARD on config pins. A pid alone cannot say whether the process
holding a pin is still the one that took it - pids are recycled, and a long-lived host
will outlive plenty of them. The start time makes the pair (pid, start_time) an identity:
the kernel will not hand out the same pid with the same start tick, so a pin whose stored
start time no longer matches belongs to a process that has already gone.

Same last-')' counting as above, so field 22 is index 19.

## struct OrphanReaper
The subreaper is taken for its ATTACHMENT property, but the same syscall imposes a DUTY:
we are now those orphans' parent, and a parent must `wait()` them. An adopted child that
exits with no waiter otherwise stays a zombie for the host's LIFETIME, holding a pid and a
process-table slot and polluting the very /proc sweeps our own orphan discipline depends
on.

The reliable producer is `git commit`, which detaches its own auto-maintenance
(`gc --auto`) - one leaked zombie per commit, and the equip lifecycle commits on every
`equip new`, `install`, `commit` and `uninstall`. It is not git-specific: any external that daemonizes
a child lands here identically.

THE RATE IS WORSE THAN ONE PER COMMIT IMPLIES, because it compounds through nested
process trees. Measured: a host at 6 zombies, and one
`cargo test -p sourcetrait_grammar_mcp` run took it to 166 - the integration suite does
dozens of in-process equip commits, and each test binary then exits, orphaning its gc
children onto the nearest subreaper ancestor, which is the host running the eval. The
same sample carried `grep` and `awk` zombies from an agent-side Monitor's tail pipeline,
putting the not-git-specific claim in evidence rather than in principle.

SERVE-PATH ONLY. A one-shot CLI process exits promptly, at which point its orphans
reparent to init and are reaped there.

### fn reap
Runs on the watchdog tick, which is cheap enough - a /proc scan of stat files, and usually
nothing to reap. `WNOHANG` so a surprise never blocks the watchdog, and a pid someone else
already reaped returns ECHILD, which is exactly the no-op wanted.

## fn install_child_subreaper
Best-effort and called once at host startup. The subreaper only buys RACE-SAFETY on
kill-time orphans; the engine's /proc descendant walk is the actual mechanism.
