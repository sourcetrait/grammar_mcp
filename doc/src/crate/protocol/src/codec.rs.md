# codec.rs

The wire contract fixes the stream's shape: `GRAMD001`, then frames of a
four-byte big-endian length and exactly that many bitcode bytes, with no
compression. This file is that shape and nothing about a transport, so a russh
channel, a Unix socket, and a test duplex all carry it alike.

## const PREFACE
Each direction writes it before its first frame, so either end that reads
anything else, a shell's banner on an SSH session among them, refuses the
stream before it dispatches a byte.

## const CODEC_ID
The hello names it, and a test holds it to the bitcode version `Cargo.lock`
resolves, so a dependency bump that changes the format cannot pass silently:
the identity changes with it, and an old peer refuses the new one.

## const SCHEMA_ID
The hellos name it, and v1 requires exact agreement, as with the codec. It is
the SHA-256 of the schema's raw NUON with every record's keys sorted, the text
nushell's `to nuon --raw` writes for the same value, not of the file's bytes,
so a checkout's line endings and the file's layout never change it, and
nushell checks it in one line. `tests/framing.rs` generates `schema/v1.nuon`
from source with syn, as the four flat tables `schema/Schema.nutype` types:
every public type bitcode encodes in the protocol and nuval, its variants and
every field in declaration order with their types, then the aliases and
datum's types those fields reach, a field bitcode skips left out. The
generator refuses what would let the layout change unseen: a cfg on a field
or a variant, an array length other than a literal, and a hand-written bitcode
impl. `BLESS=1` rewrites the file, and the constant then takes the new ID. It
holds 32 bytes rather than hex text, so the hello's field is fixed width.

## fn schema_id
A const fn, so a malformed literal fails the build rather than a hello.

## fn check_codec
Exact agreement alone, as the contract asks for v1. A compressed variant is
just another id, so it is refused like any other.

## enum FrameError
Each failure closes its own stream and nothing else. A received frame past the
ceiling and a frame too large to send are apart: the first is the peer's
invalid frame, the second this end's result too large, and the transport
errors they map to differ.

### fn transport
The stable `daemon::` kind a frontend reports, so a framing failure reaches the
envelope as the other transport failures do.

## struct FrameCodec
One codec a stream, holding the parse across partial reads: how much of the
preface has arrived, and a body's length once its header has. Every stream
starts at the hello's ceiling, since its first frame is the hello whatever its
role; the caller sets the role's negotiated ceiling once the handshake ends.

### fn decode
The preface is checked a byte at a time as it arrives, so a wrong stream is
refused at its first wrong byte, not after eight. A header's length is checked
against the ceiling before anything is reserved, and the reservation is then
that checked length, so a hostile length costs nothing.

### fn decode_eof
A clean end is one at a frame boundary after the preface. Every other end says
where it fell, in the preface, a header, or a body, with what was held and what
was owed, so a truncation is told apart from a peer that simply left.

### fn encode
The preface goes out before the first frame alone. An empty payload and one
past the ceiling are refused before anything is written, so a refused frame
leaves the stream untouched.

## fn framed
The one adapter: tokio-util's `Framed` over any `AsyncRead + AsyncWrite`. The
transport's authentication stays the transport's.

## fn payload
Encoding is the pinned bitcode's, which is deterministic, so a frame's bytes
are the same on every platform and golden vectors hold everywhere.

## trait BoundedDecode
bitcode's decode refuses trailing bytes and unknown variants, but it allocates
by the lengths a payload declares as it reads, so a small frame can declare a
huge collection. A payload therefore becomes a value only through this trait:
DecodeBudget implements it for the transport's types with budgets checked
before allocation, and the fixed-width types implement it here, since their
decode allocates nothing by a declared length.

## fn decode_exact
Crate-private, so no unbounded decode is public: only a `BoundedDecode`
implementation reaches it.

## macro fixed_width
The identities, the epoch, the lane, and the emergency kind: fixed widths
alone, the bounded decode's trivial case and the framing tests' way to reach
the trailing-byte and unknown-variant refusals.
