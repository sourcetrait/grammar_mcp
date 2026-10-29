# capability.rs

## struct Capabilities
One flag a part of the daemon split that reaches users through its own gate:
the wire, the runtime, the client, artifacts, and the service. A build
advertises a flag only once that part's gate passes, so a peer never relies on
a part that exists only as a home in the tree.

The set is cereal data because it crosses the wire in the hello, and it is
`Copy` because it is five booleans.

## const NONE
Every flag off: what a build advertises before any gate passes, and what the
client, the runtime, and the probe's inventory report for now.
