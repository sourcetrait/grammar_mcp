//! The wire's framing: the preface, the length-prefixed frames, their ceilings,
//! how a stream may end, and the bounded decode's refusals.
use futures_util::{SinkExt, StreamExt};
use sourcetrait_grammar_protocol::{
    CODEC_ID, EVENT_FRAME_MAX, EmergencyKind, FrameCodec, FrameError, FrameStage, HELLO_FRAME_MAX,
    Lane, OperationId, PREFACE, RPC_FRAME_DEFAULT, RPC_FRAME_HARD, StreamRole, TransportError,
    check_codec, decode_payload, framed, payload,
};
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// The wire bytes of `frames`, the preface first.
fn wire(frames: &[&[u8]]) -> Vec<u8> {
    let mut out = PREFACE.to_vec();
    for frame in frames {
        out.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        out.extend_from_slice(frame);
    }
    out
}

/// Feed `bytes` to a fresh codec one byte at a time, collecting the frames.
fn feed_bytewise(bytes: &[u8]) -> Result<Vec<Bytes>, FrameError> {
    let mut codec = FrameCodec::new();
    let mut buf = BytesMut::new();
    let mut frames = Vec::new();
    for byte in bytes {
        buf.extend_from_slice(&[*byte]);
        if let Some(frame) = codec.decode(&mut buf)? {
            frames.push(frame);
        }
    }
    Ok(frames)
}

/// Decode `bytes` whole, then end the stream.
fn decode_to_end(codec: &mut FrameCodec, bytes: &[u8]) -> Result<Vec<Bytes>, FrameError> {
    let mut buf = BytesMut::from(bytes);
    let mut frames = Vec::new();
    while let Some(frame) = codec.decode(&mut buf)? {
        frames.push(frame);
    }
    while let Some(frame) = codec.decode_eof(&mut buf)? {
        frames.push(frame);
    }
    Ok(frames)
}

#[test]
fn a_frame_is_the_preface_a_big_endian_length_and_its_bitcode() {
    let mut codec = FrameCodec::new();
    let mut out = BytesMut::new();
    let value = OperationId([7; 16]);
    codec.encode(payload(&value), &mut out).expect("encoded");
    codec.encode(payload(&value), &mut out).expect("encoded again");
    // The preface goes out once, before the first frame alone.
    let frame = bitcode::encode(&value);
    assert_eq!(out.as_ref(), wire(&[&frame, &frame]).as_slice());
    // Golden: bitcode packs the sixteen equal bytes into four, the same on
    // every platform.
    let golden = "4752414d4430303100000004090700000000000409070000";
    let hex: String = out.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, golden);
    let back = decode_to_end(&mut FrameCodec::new(), &out).expect("decoded");
    assert_eq!(back.len(), 2);
    assert_eq!(decode_payload::<OperationId>(&back[0]).expect("an id"), value);
}

#[test]
fn one_byte_reads_assemble_each_frame() {
    let bytes = wire(&[b"alpha", b"be"]);
    let frames = feed_bytewise(&bytes).expect("framed");
    assert_eq!(frames, vec![Bytes::from_static(b"alpha"), Bytes::from_static(b"be")]);
}

#[test]
fn coalesced_frames_come_apart_in_order() {
    let bytes = wire(&[b"one", b"two", b"three"]);
    let frames = decode_to_end(&mut FrameCodec::new(), &bytes).expect("framed");
    assert_eq!(frames, vec![Bytes::from_static(b"one"), Bytes::from_static(b"two"), Bytes::from_static(b"three")]);
}

#[test]
fn a_bad_preface_is_refused_at_its_first_wrong_byte() {
    // Five bytes are enough: the decoder does not wait for eight.
    let err = feed_bytewise(b"GRAMX").expect_err("refused");
    assert!(matches!(err, FrameError::BadPreface { at: 4, got: b'X' }), "{err}");
    assert_eq!(err.transport().kind(), "daemon::invalid_frame");
}

#[test]
fn an_empty_stream_closed_before_its_preface() {
    let err = decode_to_end(&mut FrameCodec::new(), b"").expect_err("refused");
    assert!(matches!(err, FrameError::ClosedBeforePreface), "{err}");
}

#[test]
fn a_stream_ending_inside_a_preface_a_header_or_a_body_says_where() {
    let full = wire(&[b"0123456789"]);
    let cases = [
        (5, FrameStage::Preface, 5, 8),
        (10, FrameStage::Header, 2, 4),
        (15, FrameStage::Body, 3, 10),
    ];
    for (cut, stage, held, expected) in cases {
        let err = decode_to_end(&mut FrameCodec::new(), &full[..cut]).expect_err("truncated");
        match err {
            FrameError::Truncated { stage: s, held: h, expected: e } => {
                assert_eq!((s, h, e), (stage, held, expected), "cut at {cut}");
            }
            other => panic!("cut at {cut}: {other}"),
        }
    }
}

#[test]
fn a_stream_ending_at_a_frame_boundary_ends_cleanly() {
    let frames = decode_to_end(&mut FrameCodec::new(), &wire(&[b"done"])).expect("clean");
    assert_eq!(frames, vec![Bytes::from_static(b"done")]);
    // The preface alone, then an end, is a clean end too.
    assert!(decode_to_end(&mut FrameCodec::new(), &PREFACE).expect("clean").is_empty());
}

/// A header declaring `len`, after the preface.
fn header(len: u32) -> BytesMut {
    let mut buf = BytesMut::from(&PREFACE[..]);
    buf.extend_from_slice(&len.to_be_bytes());
    buf
}

#[test]
fn zero_and_oversize_lengths_are_refused_before_any_reservation() {
    let mut codec = FrameCodec::new();
    let err = codec.decode(&mut header(0)).expect_err("refused");
    assert!(matches!(err, FrameError::ZeroLength), "{err}");
    for len in [HELLO_FRAME_MAX + 1, u32::MAX] {
        let mut codec = FrameCodec::new();
        let mut buf = header(len);
        let err = codec.decode(&mut buf).expect_err("refused");
        assert!(matches!(err, FrameError::Oversize { len: l, ceiling } if l == len && ceiling == HELLO_FRAME_MAX), "{err}");
        // Refused at the header: nothing near the declared length was reserved.
        assert!(buf.capacity() < 64, "capacity {}", buf.capacity());
    }
}

#[test]
fn each_ceiling_holds_exactly_and_refuses_one_past_it() {
    for (role_ceiling, set) in [
        (HELLO_FRAME_MAX, None),
        (EVENT_FRAME_MAX, Some(StreamRole::Events.default_ceiling())),
        (RPC_FRAME_DEFAULT, Some(StreamRole::Rpc.default_ceiling())),
    ] {
        let mut codec = FrameCodec::new();
        // The hello comes first on every stream, under its own ceiling.
        let mut buf = BytesMut::from(wire(&[b"hello"]).as_slice());
        assert!(codec.decode(&mut buf).expect("hello").is_some());
        if let Some(ceiling) = set {
            codec.set_ceiling(ceiling);
        }
        let mut buf = BytesMut::from(&(role_ceiling).to_be_bytes()[..]);
        buf.extend_from_slice(&vec![1u8; role_ceiling as usize]);
        let frame = codec.decode(&mut buf).expect("at the ceiling").expect("whole");
        assert_eq!(frame.len(), role_ceiling as usize);
        let err = codec.decode(&mut BytesMut::from(&(role_ceiling + 1).to_be_bytes()[..])).expect_err("past it");
        assert!(matches!(err, FrameError::Oversize { .. }), "{err}");
    }
    assert_eq!(StreamRole::Status.default_ceiling(), RPC_FRAME_DEFAULT);
    assert_eq!(StreamRole::Rpc.hard_ceiling(), RPC_FRAME_HARD);
    assert_eq!(StreamRole::Events.hard_ceiling(), EVENT_FRAME_MAX);
}

#[test]
fn a_frame_to_send_must_hold_something_and_fit_the_ceiling() {
    let mut codec = FrameCodec::new();
    let mut out = BytesMut::new();
    let err = codec.encode(Bytes::new(), &mut out).expect_err("empty");
    assert!(matches!(err, FrameError::ZeroLength), "{err}");
    let big = Bytes::from(vec![0u8; HELLO_FRAME_MAX as usize + 1]);
    let err = codec.encode(big, &mut out).expect_err("too large");
    assert!(matches!(err, FrameError::TooLarge { .. }), "{err}");
    assert!(matches!(err.transport(), TransportError::ResultTooLarge { limit } if limit == u64::from(HELLO_FRAME_MAX)));
    assert!(out.is_empty(), "a refused frame writes nothing, the preface included");
}

#[test]
fn the_codec_is_the_bitcode_the_lock_resolves() {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"))
        .expect("the workspace's Cargo.lock");
    let version = lock
        .split("[[package]]")
        .find(|block| block.contains("name = \"bitcode\""))
        .and_then(|block| block.lines().find_map(|l| l.strip_prefix("version = \"")))
        .and_then(|v| v.strip_suffix('"'))
        .expect("bitcode in the lock");
    assert_eq!(CODEC_ID, format!("bitcode-{version}"));
    assert!(check_codec(CODEC_ID).is_ok());
    for other in ["bitcode-0.6.9+zstd", "bitcode-0.6.8", ""] {
        let err = check_codec(other).expect_err("refused");
        assert_eq!(err.transport().kind(), "daemon::codec_mismatch");
    }
}

/// One variant more than EmergencyKind, in the same four-bit packing.
#[allow(dead_code)]
#[derive(bitcode::Encode)]
enum TenKinds {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
}

#[test]
fn a_bounded_decode_refuses_trailing_bytes_and_unknown_variants() {
    let id = OperationId([3; 16]);
    let mut bytes = bitcode::encode(&id);
    assert_eq!(decode_payload::<OperationId>(&bytes).expect("whole"), id);
    bytes.push(0);
    assert!(matches!(decode_payload::<OperationId>(&bytes), Err(FrameError::Decode)));
    assert_eq!(decode_payload::<Lane>(&bitcode::encode(&Lane::Stateful)).expect("a lane"), Lane::Stateful);
    // A Lane's two variants fill every index its encoding can hold. An
    // EmergencyKind's nine pack into four bits as ten do, so a ten-variant
    // enum's last encodes an index that names none of the nine.
    let kind = EmergencyKind::Critical;
    assert_eq!(decode_payload::<EmergencyKind>(&bitcode::encode(&kind)).expect("a kind"), kind);
    let unknown = bitcode::encode(&TenKinds::J);
    assert!(matches!(decode_payload::<EmergencyKind>(&unknown), Err(FrameError::Decode)));
}

#[tokio::test]
async fn the_adapter_carries_frames_over_a_byte_stream_a_byte_at_a_time() {
    // A one-byte duplex hands every read a single byte.
    let (near, far) = tokio::io::duplex(1);
    let mut sender = framed(near);
    let mut receiver = framed(far);
    let sent = vec![payload(&OperationId([1; 16])), payload(&OperationId([2; 16]))];
    let send = tokio::spawn(async move {
        for frame in sent {
            sender.send(frame).await.expect("sent");
        }
    });
    let mut got = Vec::new();
    while got.len() < 2 {
        let frame = receiver.next().await.expect("a frame").expect("framed");
        got.push(decode_payload::<OperationId>(&frame).expect("an id"));
    }
    send.await.expect("sender");
    assert_eq!(got, vec![OperationId([1; 16]), OperationId([2; 16])]);
}
