# system.rs

mcp as a subsys system, the manager of the others: it starts the engine and, on the
serve path, the operator; the tools' requests reach the engine through it, and the
engine's host asks go up through it to its controller; it routes the engine's health to
the operator and the operator's emergencies to the Channel; it hears each component's
status and end; and it stops them in order.

The tools' requests go through it, which was settled by measurement: a subsys round trip
costs about ten microseconds in a release build, against about a millisecond and a half
for the cheapest `execute` call, so the hop adds well under one percent to the cheapest
call and less to any real one.

## enum Notice
What a component's handler hears of it, carried to mcp's loop, since a handler runs on
its component's control task and the loop is where mcp acts.

## struct EngineWatch
The engine's status changes and an end nobody asked for go to mcp's loop. Its host
asks go up through mcp's system's own requester to that system's controller
(`host.rs`'s `HostRoute`), which holds the face's systems: a subsys system takes only
data at its start, so mcp's system cannot hold them itself. An ask its controller
cannot answer is refused in the controller's words, so an eval never waits on one.

## struct McpSysParams
### field supervise
The serve path's supervision, the operator, its observer, and the emergency lane, which
the one-shot CLI and the in-process test harness leave out: a one-shot exits before any
of it would matter, and the harness would otherwise take the namespace's host lock and
sample the test process.

## enum ToMcpSys
The engine's own requests wrapped whole, so a tool sends the same request types it sent
the engine, each with its own answer type, the wire's and the engine's purview requests
alike; and the controller's answer to a host ask.

## fn forward_purview
A purview request goes on to the engine as a wire request does, and one the engine could
not take answers its failure in the request's own kind (`purview_refused`), so a tool
never waits on one mcp dropped.

## enum FromMcpSys
The engine's answers wrapped whole, the shutdown's, and an engine's host ask on its way
up, the same ask the engine made, so its answer needs no translating either way.

## fn forward
Every request is answered. A request the engine couldn't take answers its failure under
its own nonce, so a tool never waits on one mcp dropped.

## const ENGINE_RELEASE_GRACE
After the teardown cancels the calls in flight, their requests let go of the engine
within milliseconds; one whose eval is hung where nushell never checks never does, so
the wait is bounded and the engine is left to the process's exit.

## fn McpSystem::shut_down
The order is the shutdown's point. The observer stops first, so nothing observes a
stopping engine; the engine's `Teardown` cancels the calls in flight and reaps their
trees; the operator stops next, its host lock released as late as anything that still
works for the host; and the engine's system stops last, once no request holds it.

Once only: the shutdown request and the system's own stop both run it.

## fn McpSystem::on_notice
A component's end is logged, not acted on: no restart is designed, and the requests a
stopped engine can't answer already answer their failure. An end during the shutdown is
the shutdown's own and goes unsaid.

## fn McpSystem::init
The emergency lane is built here, before the operator, because the operator's
emergencies are routed onto it and there is nothing to push onto until it exists. The
lane is installed into the running face's hub slot (`server/channel/bridge.rs`), whose
pump puts the Channel's spam pair on it; the face starts before the system, so the slot
is there to take it.

## fn McpSystem::on_channel_recv
A request for the engine is answered from a task of its own, so a long eval holds no
other request up; the shutdown runs on the loop itself, so nothing is forwarded to an
engine part-way through stopping. A host answer reaching here answers no waiter, its
ask gone, and is dropped.

## fn McpHandle::shutdown
A request rather than a stop: the system serves on until the process ends, so a request
arriving late answers the engine's absence rather than failing to send.

## fn start_system
The config's lines and `--test` flag are the operator's parameters; the system reads no
config of its own once started. `route` is the controller's handler, holding the face's
systems, so every start of mcp's system has a face beside it.
