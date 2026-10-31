# wire.rs

The Qualification contract's `wire` group: every v1 frame and variant, the
limits, malformed and truncated input, and the independent golden vectors. It
needs no environment, so it runs wherever the probe does.

## const FILES / const MANIFEST
The vector files and the manifest are compiled in from the protocol crate's
`tests/vectors/`, so the probe carries the bytes it was built with: a probe
built at the freeze holds the frozen set, and a protocol built later is held
to it. The bytes are decoded through the protocol's fixture driver, the DTOs
being the contract both ends share, never through the protocol's tests.

## const ASSERTIONS
The group's enumerated assertions, in the order they run:

- `wire.manifest`: the manifest names this build's version, schema ID, and
  codec, and pins each file's canonical digest and vector count.
- `wire.vectors`: every vector decodes by its type to its own bytes, and each
  truncation and a trailing byte are refused.
- `wire.types`: every driver type names a vector, and every vector's type a
  driver.
- `wire.limits`: every vector fits its stream's frame, and each hello its
  budget.
- `wire.handshake`: the frozen hello is accepted, and one of another major,
  schema, or codec is refused with its kind, binding no lease and admitting
  no request.
- `wire.execution`: no submission, whole, cut short, or mutated, runs or is
  admitted without a ticket the worker issued.

## fn run
The first failure stops the group and names its assertion, so a result lists
the assertions run through it.
