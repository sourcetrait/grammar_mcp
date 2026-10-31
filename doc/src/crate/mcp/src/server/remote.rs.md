# remote.rs

The controller's handler of the remote engines' system (`crate/client`'s
`system.rs`): a worker's asks come up as requests it answers, its emergencies
as packets, and the system's status and end are logged as the engine's are.

## struct RemoteRoute
### field stopping
The controller's shutdown flag, under which the system's end goes unsaid.

### fn on_packet
A worker's emergency is logged under its engine's alias, by its kind.

### fn on_request
A worker's ask is refused with why, so the eval that asked answers its
refusal at once rather than at its deadline.

### fn on_sub
### fn on_closed
Logged as the engine's are (`server/controller.rs`).
