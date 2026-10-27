# msg.rs

The operator's protocol with its manager, every type cereal data. Both directions are
SINGULAR packets, no request waiting on an answer: an observation is the manager's
news, and an emergency the operator's.

## enum ThreadLane
The operator depends on no engine, so a watched thread's lane is its own type; the
manager maps the engine's lane onto it.

## struct EngineReading
The engine as the supervisor reads it, in the operator's own terms: the watched
threads, the permits held against the cap, and the jobs table's size. The manager
builds it from the engine's health.

## struct Observation
### field engine
None when the engine didn't answer in time. That is an ABSENT reading, not an empty
one: the watchdog keeps its active conditions across it rather than clearing them.

### field listening
The Channel is the host's, so its state reaches the operator as news; a test host
samples only while it is open.
