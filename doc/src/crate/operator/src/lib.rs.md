# lib.rs

## use sourcetrait_common::cereal
The emergencies and the protocol are cereal data, so the operator takes `::serde`,
`::bitcode`, and `::rkyv` as dependencies of its own, the paths `cereal::derived` emits,
as the engine does.

## mod tk
tokio's items the system's loop names: its `select!`, its interval, the probes' blocking
threads, and the channel they answer on.

## pub use
What a manager names: the system, its handle, and its parameters and paths; the
protocol, the observation it passes and the emergency it gets back, with every
emergency's struct; the supervisor lines and their range check; the pins, which the
host's evals pin through; a process's start time, the identity the pins and the host's
scratch directories share; the subreaper; and the interval the manager observes at.
Everything else is the crate's own.
