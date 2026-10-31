# run.rs

## fn run_main
The error's text goes to stderr behind `grammard: ` and the process exits by
the error's code, so a stub command reads as unimplemented to a person and to
a script alike, and stdout stays empty for the commands that will speak a
protocol on it.

## fn run
`serve` runs a worker in the foreground; every other command refuses with
`Unimplemented`, since a command that exits 0 before it exists would read as
one that works. Each replaces its refusal as its own code lands; the local
CLI's already runs through its declared stub (`cli/oneshot.rs`).

## fn serve
The stop signals are blocked before anything starts, so every thread the start
spawns inherits the block and the main thread alone takes a signal, by
`sigwait`, then shuts the runtime down. The environment is read once as data,
an entry that isn't UTF-8 left out. An instance with no runtime home has no
socket to serve, and is refused before anything starts. The ready line on
stdout, naming the instance, the generation, the pid, and the socket, comes
once the socket is bound: a foreground start's readiness, a service's coming
with its activation.

## fn stop_signals
SIGTERM, a service manager's stop; SIGINT, a terminal's; SIGHUP, a closed
session's.
