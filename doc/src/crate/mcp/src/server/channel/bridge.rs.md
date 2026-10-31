# bridge.rs

The running face's Channel, held for the code not yet a system or a
controller's: the engine's host (`server/host.rs`), the remote links
(`server/remote/link.rs`), and mcp's system's emergency lane, responder, and
observer (`server/system.rs`, `server/emergency.rs`, `server/operator.rs`).
Each is handed its own as SubsysExtraction reaches it - the links by
PeerSystem, the host by HostRoute, the lane and its responder by
EmergencyPolicy, the observer by McpController - and the bridge goes with the
last of them.

## static HOST_CHANNEL
The face started last. A process serves one face, so it is the one the agent
talks to; a test process starts a face for each test server, and the embedded
tests take their turns one at a time, so the newest is the running test's.

## struct HostChannel
### field posts
The remote links emit from synchronous code inside their async drivers, so a
relay or a report is posted here, never awaited, and one task emits the posts
in the order they were made.

## fn drain_posts
A relay that fails is logged by its delivery's id, as the link logged it; a
report is best effort and fails silently.
