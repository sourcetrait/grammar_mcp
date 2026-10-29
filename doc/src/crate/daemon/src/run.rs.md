# run.rs

## fn run_main
The error's text goes to stderr behind `grammard: ` and the process exits by
the error's code, so a stub command reads as unimplemented to a person and to
a script alike, and stdout stays empty for the commands that will speak a
protocol on it.

## fn run
Every command refuses with `Unimplemented`: a command that exits 0 before it
exists would read as one that works. Each command replaces its refusal as its
own code lands.
