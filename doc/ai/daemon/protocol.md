# Grammar daemon protocol v1: gate receipt

The frozen contract the runtime and the portable clients implement against.
A change to anything below changes the schema, the vectors, or both, and is
made on purpose, both ends together, never one end tolerating the other.

## Identity

- Protocol version 1.0, `PROTOCOL_VERSION`. Both hellos name it, and the major
  must match exactly.
- Schema `crate/protocol/schema/v1.nuon`, generated from the source of every
  public wire type: 159 types, 580 variants, and 572 fields, each variant's
  index and each field's order and type recorded, and every field of a fixed
  width. Its ID, `SCHEMA_ID`, is the SHA-256 of its canonical form:
  `ed5e410c9474e8e520d4d399f081745e07a42edd0615ad3971cc8cc4428d08cb`.
- Codec `bitcode-0.6.9`, `CODEC_ID`: crates.io's bitcode 0.6.9, replaced by
  the workspace's `[patch.crates-io]` with the sibling fork at commit
  `62de42a`, which adds the decode budget and a sound copy and encodes the
  same bytes as upstream 0.6.9 across a differential run of 76,000 values.
- Fixture manifest `crate/protocol/tests/vectors/manifest.nuon`: the version,
  the schema ID, the codec, and ten vector files of 326 vectors, each file
  pinned by the SHA-256 of its table's canonical NUON, so a checkout's line
  endings and a file's layout move nothing.

## Streams

Every stream opens with the eight bytes `GRAMD001`, then frames of a 4-byte
big-endian length and exactly that many bitcode bytes. A stream's role is
fixed by its hello and reads one frame type a direction:

| Role | Client to worker | Worker to client | Frame ceiling |
|---|---|---|---|
| any, first frame | `ClientHello` | `HelloReply` | 64 KiB, `HELLO_BUDGET` |
| RPC | `RpcFrame`: ticket, submit, status, lease, ping, close | `RpcAnswer`: ticket, admission, response, status, lease, pong, close | negotiated, 8 MiB by default, 64 MiB at most |
| Events | `EventsFrame`: resume, ack, ping, close | `EventsAnswer`: event, pong, close | 1 MiB |
| Status | `StatusFrame`: lookup, health, ping, close | `StatusAnswer`: lookup, health, pong, close | as RPC |

A runtime request is `RuntimeRequest`: a typed engine request, an interface
command's raw text the runtime parses, info, an operation's lookup, the
controller's policy, a Channel notice, an artifact request, or a
cancellation. A close is an orderly detach and never shuts the worker down.

## Compatibility and rejection

- A hello of another major, schema, or codec is refused before any lease
  binds: `daemon::version_mismatch`, `daemon::schema_mismatch`,
  `daemon::codec_mismatch`. A required feature that isn't on is
  `daemon::unsupported_feature`; another worker's identity is
  `daemon::wrong_worker`. A malformed or second hello, an intent the role
  doesn't allow, a bound at zero or raised past the request or the hard
  ceilings, a key that isn't the instance's, and an artifact bound out of
  contract are each `daemon::invalid_frame`. A refused hello's stream admits
  nothing.
- A frame of zero length or past its stream's ceiling, a stream ending inside
  a preface, a header, or a body, and a payload with trailing bytes or an
  unknown variant are refused, each closing its own stream.
- A payload decodes within its budget, checked before what it bounds is
  allocated: 100,000 elements, 32 MiB of heap by default and 128 MiB at most,
  and a structural depth of 128, a value's own 64 levels held by
  `check_value`.
- Nothing runs without a ticket the worker issued, unexpired, unused, of its
  generation and the lease's epoch, under the request's own payload hash:
  `daemon::ticket_invalid` otherwise, and `daemon::operation_conflict` for an
  operation admitted before with another payload.
- A peer's packet is data: its nom and delivery id are base62, its model
  outside the `mcp/` and `remote/` reservations, and its files' destinations
  held to every client OS's rules before any event is numbered.

## Limits

| Bound | Default | Hard |
|---|---:|---:|
| Hello frame | 64 KiB | 64 KiB |
| RPC and Status frame | 8 MiB | 64 MiB |
| Event frame | 1 MiB | 1 MiB |
| Decoded heap per frame | 32 MiB | 128 MiB |
| Decoded elements / structural depth | 100,000 / 128 | the same |
| Ordinary RPCs in flight per lease | 32 | 128 |
| Control and status slots | 4 | 8 |
| Event backlog | 4,096 records, 16 MiB | 65,536 records, 256 MiB |
| Transport streams per worker | 16 | 64 |
| Tickets awaiting submission | 1,024, each 60 s | the same |
| Lease heartbeat / expiry | 10 s / 30 s | the same |
| Hello / authentication | 5 s / 30 s | the same |
| Artifact, reservations, store | 256 MiB, 8, 4 GiB | the same until qualified |
| Reservation life / retention | 1 h / 7 days | 1 h / no ceiling, 7 days the floor |
| SFTP packet / data chunk / log read | 1 MiB / 64 KiB / 64 KiB | fixed |

A hello negotiates the RPC frame, the decoded heap, the RPCs in flight, the
control slots, the event backlog, and the transport streams, each to the least
of the client's request, the worker's own, and the hard ceiling. The worker
advertises the artifact store's bounds, and the rest are fixed.

## Checks

On the commit that froze this receipt:

- `cargo test --locked -p sourcetrait_grammar_protocol --lib --tests -- --test-threads=1`:
  142 tests over 15 targets pass, among them `gate`, which decodes every
  vector by its type through the fixture driver to its own bytes, refuses
  each truncation and a trailing byte, decodes random and mutated bytes
  within the budget to canonical forms, admits no malformed submission, and
  holds the manifest.
- `cargo clippy --locked -p sourcetrait_grammar_protocol --all-targets`: clean.
- `grammard-probe run --case wire`: its six assertions pass against the
  vectors and manifest compiled into it.
- The workspace battery at `64c31e9`, both cohorts' builds, tests, and lints,
  passes on a Linux host and a macOS guest: the client cohort's 399 crate tests
  over 81 targets and 29 system tests, the embedded cohort's 378 crate tests
  and 112 system tests on Linux and 109 on macOS, and clippy at its baseline.

## The fixture driver

`sourcetrait_grammar_protocol::fixture` decodes a vector by the type its row
names and encodes it back, reads a vectors file, digests a file, and checks a
vector's round trip and malformed forms. A native client's or daemon's tests,
and the probe, take the same bytes through it, sharing the DTOs alone.

## Open items

- The Runtime task schedules control frames into the reserved slots, and
  enforces the transport-stream bound at accept with `daemon::busy`.
- The artifact store's bounds above their defaults wait on a qualification
  run that proves them.
