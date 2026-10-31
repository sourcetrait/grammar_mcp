# bridge.rs

The running face's Channel, held for the code not yet a system or a controller's:
mcp's system's emergency lane, responder, and observer (`server/system.rs`,
`server/emergency.rs`, `server/operator.rs`). Each is handed its own as
SubsysExtraction reaches it - the lane and its responder by EmergencyPolicy, the
observer by McpController - and the bridge goes with the last of them.

## static HOST_CHANNEL
The face started last. A process serves one face, so it is the one the agent
talks to; a test process starts a face for each test server, and the embedded
tests take their turns one at a time, so the newest is the running test's.

## struct HostChannel
The Channel's handle and the hub, cloned out whole, so a reader holds the two of
one face.
