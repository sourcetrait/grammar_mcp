# bridge.rs

The running face's Channel, held for the code not yet a system or a
controller's: mcp's system's observer (`server/operator.rs`), which McpController
hands its own, and the bridge goes with it.

## static HOST_CHANNEL
The face started last. A process serves one face, so it is the one the agent
talks to; a test process starts a face for each test server, and the embedded
tests take their turns one at a time, so the newest is the running test's.

## struct HostChannel
The Channel's handle, cloned out, as the observer reads its phase.
