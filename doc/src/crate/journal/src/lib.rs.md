# lib.rs

The operation journal as a subsys system, in a crate of its own so a product
takes it without the daemon (User). The protocol's `OperationTable` is its
model; the system adds the clock, the ids a ticket draws, and the files that
make each move durable.

## mod guts
The journal itself and its fault plan, public for the crate's tests alone,
which drive the journal without a runtime and arm a fault at any write.

## pub use
The system, its paths, params, config, and handle; its requests and their
answers; the journal's errors; and its two bounds, the cap and the inline
outcome's.
