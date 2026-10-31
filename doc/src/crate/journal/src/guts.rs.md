# guts.rs

The journal and its fault plan for the tests: a fault armed at a write, a sync,
a rename, or a directory's sync reaches each boundary between two durable
states, with no seam in the I/O beyond the plan's points.
