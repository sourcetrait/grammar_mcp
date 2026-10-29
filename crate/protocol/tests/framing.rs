//! The wire's framing: the preface, the length-prefixed frames, their ceilings,
//! how a stream may end, the bounded decode's refusals, and the schema and its ID.
use std::path::Path;

use futures_util::{SinkExt, StreamExt};
use sourcetrait_grammar_protocol::{
    CODEC_ID, EVENT_FRAME_MAX, EmergencyKind, FrameCodec, FrameError, FrameStage, HELLO_FRAME_MAX,
    Lane, OperationId, PREFACE, RPC_FRAME_DEFAULT, RPC_FRAME_HARD, SCHEMA_ID, StreamRole,
    TransportError, check_codec, decode_payload, framed, payload,
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

/// The wire's schema: every public type bitcode encodes in the protocol and
/// nuval, read from their source, with the canonical form its ID hashes.
mod schema {
    use std::collections::BTreeMap;
    use std::path::Path;

    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use syn::punctuated::Punctuated;
    use syn::{Meta, Token};

    /// The checked-in schema, under the protocol crate.
    pub const FILE: &str = "schema/v1.json";

    /// The crates whose source declares the wire's own types, by directory.
    const CRATES: [&str; 2] = ["protocol", "nuval"];

    /// Common's datum, the workspace's sibling, whose types the wire carries
    /// where a field names them.
    const DATUM: &str = "../../../common/crates/sourcetrait_datum/src";

    /// A schema's records by name.
    type Records = BTreeMap<String, Value>;

    /// The schema as the source declares it: each type, and each alias and
    /// datum type the types' fields reach.
    pub fn generate() -> Value {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut types = Records::new();
        let mut reachable = Records::new();
        for name in CRATES {
            let src = manifest.join("..").join(name).join("src");
            walk_file(&src.join("lib.rs"), &src, &mut types, &mut reachable);
        }
        let mut datum = Records::new();
        let src = manifest.join(DATUM);
        walk_file(&src.join("lib.rs"), &src, &mut datum, &mut reachable);
        for (name, shape) in datum {
            record(&mut reachable, name, shape);
        }
        let mut aliases = Records::new();
        for (name, shape) in reach(&types, &reachable) {
            match shape.get("kind") {
                Some(_) => record(&mut types, name, shape),
                None => record(&mut aliases, name, shape),
            }
        }
        json!({
            "aliases": Value::Object(aliases.into_iter().collect()),
            "types": Value::Object(types.into_iter().collect()),
        })
    }

    /// The SHA-256 of a schema text's canonical form.
    pub fn id(text: &str) -> [u8; 32] {
        let value: Value = serde_json::from_str(text).expect("the schema's JSON");
        let mut id = [0u8; 32];
        id.copy_from_slice(&Sha256::digest(canonical(&value).as_bytes()));
        id
    }

    /// The form the ID hashes: keys sorted, no whitespace, UTF-8.
    pub fn canonical(value: &Value) -> String {
        let mut out = String::new();
        write(value, false, 0, &mut out);
        out
    }

    /// The checked-in form: the canonical order, two-space indents, LF, and
    /// a record holding nothing nested on one line.
    pub fn pretty(value: &Value) -> String {
        let mut out = String::new();
        write(value, true, 0, &mut out);
        out.push('\n');
        out
    }

    /// One value, its objects' keys sorted.
    fn write(value: &Value, pretty: bool, depth: usize, out: &mut String) {
        let entries: Vec<(Option<&String>, &Value)> = match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                keys.into_iter().map(|k| (Some(k), &map[k])).collect()
            }
            Value::Array(items) => items.iter().map(|v| (None, v)).collect(),
            scalar => {
                out.push_str(&serde_json::to_string(scalar).expect("a scalar"));
                return;
            }
        };
        let (open, close) = if value.is_object() { ('{', '}') } else { ('[', ']') };
        let filled = |v: &Value| match v {
            Value::Object(map) => !map.is_empty(),
            Value::Array(items) => !items.is_empty(),
            _ => false,
        };
        let lines = pretty && entries.iter().any(|(_, v)| filled(v));
        let indent = |out: &mut String, depth: usize| {
            out.push('\n');
            out.push_str(&"  ".repeat(depth));
        };
        out.push(open);
        for (i, (key, item)) in entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
                if pretty && !lines {
                    out.push(' ');
                }
            }
            if lines {
                indent(out, depth + 1);
            }
            if let Some(key) = key {
                out.push_str(&serde_json::to_string(key).expect("a key"));
                out.push(':');
                if pretty {
                    out.push(' ');
                }
            }
            write(item, pretty, depth + 1, out);
        }
        if lines {
            indent(out, depth);
        }
        out.push(close);
    }

    /// The file's items, its child modules' files under `dir`.
    fn walk_file(file: &Path, dir: &Path, types: &mut Records, aliases: &mut Records) {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let parsed =
            syn::parse_file(&text).unwrap_or_else(|e| panic!("parse {}: {e}", file.display()));
        walk(&parsed.items, dir, types, aliases);
    }

    /// Each public wire type and alias, modules followed, test modules skipped.
    fn walk(items: &[syn::Item], dir: &Path, types: &mut Records, aliases: &mut Records) {
        for item in items {
            match item {
                syn::Item::Mod(m) if !is_test(&m.attrs) => {
                    let child = dir.join(m.ident.to_string());
                    match &m.content {
                        Some((_, inner)) => walk(inner, &child, types, aliases),
                        None => {
                            let file = dir.join(format!("{}.rs", m.ident));
                            walk_file(&file, &child, types, aliases);
                        }
                    }
                }
                syn::Item::Struct(s) if is_public(&s.vis) && on_wire(&s.attrs) => {
                    let name = s.ident.to_string();
                    let shape = json!({
                        "kind": "struct",
                        "generics": generics(&s.generics),
                        "fields": fields(&s.fields, &name),
                    });
                    record(types, name, shape);
                }
                syn::Item::Enum(e) if is_public(&e.vis) && on_wire(&e.attrs) => {
                    let name = e.ident.to_string();
                    let variants: Vec<Value> = e
                        .variants
                        .iter()
                        .map(|v| {
                            let variant = format!("{name}::{}", v.ident);
                            fixed(&v.attrs, &variant);
                            json!({"name": v.ident.to_string(), "fields": fields(&v.fields, &variant)})
                        })
                        .collect();
                    let shape = json!({
                        "kind": "enum",
                        "generics": generics(&e.generics),
                        "variants": variants,
                    });
                    record(types, name, shape);
                }
                syn::Item::Type(t) if is_public(&t.vis) => {
                    let shape = json!({"generics": generics(&t.generics), "type": render(&t.ty)});
                    record(aliases, t.ident.to_string(), shape);
                }
                _ => {}
            }
        }
    }

    /// Records a type or an alias by its name, which the schema keeps unique.
    fn record(records: &mut Records, name: String, shape: Value) {
        if records.insert(name.clone(), shape).is_some() {
            panic!("{name} is declared twice; the schema names each type once");
        }
    }

    /// The records the types name, followed through what each of those names.
    fn reach(types: &Records, reachable: &Records) -> Records {
        let mut texts: Vec<String> = types.values().flat_map(named).collect();
        let mut reached = Records::new();
        while let Some(text) = texts.pop() {
            for word in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                if let Some(shape) = reachable.get(word)
                    && !types.contains_key(word)
                    && !reached.contains_key(word)
                {
                    texts.extend(named(shape));
                    reached.insert(word.to_string(), shape.clone());
                }
            }
        }
        reached
    }

    /// Every type a record names: an alias's own, a type's fields'.
    fn named(shape: &Value) -> Vec<String> {
        let types = |fields: &Value| -> Vec<String> {
            let fields = fields.as_array().expect("fields");
            fields.iter().map(|f| f["type"].as_str().expect("a type").to_string()).collect()
        };
        match (&shape["type"], &shape["variants"]) {
            (Value::String(ty), _) => vec![ty.clone()],
            (_, Value::Array(variants)) => variants.iter().flat_map(|v| types(&v["fields"])).collect(),
            _ => types(&shape["fields"]),
        }
    }

    /// Whether the item is `pub`, the wire's types all being public.
    fn is_public(vis: &syn::Visibility) -> bool {
        matches!(vis, syn::Visibility::Public(_))
    }

    /// Whether a `cfg` names `test`: such a module never reaches the wire.
    fn is_test(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("cfg") && words(a).iter().any(|w| w == "test"))
    }

    /// An attribute's argument words, punctuation dropped.
    fn words(attr: &syn::Attribute) -> Vec<String> {
        match &attr.meta {
            Meta::List(list) => list
                .tokens
                .to_string()
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .filter(|w| !w.is_empty())
                .map(String::from)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether bitcode's derive encodes the type: `cereal::derived` without
    /// `not(Bitcode)`, or a derive naming bitcode's `Encode`.
    fn on_wire(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| {
            let path = path_text(a.path());
            if path == "cereal::derived" {
                if cereal_listed(a, "has").iter().any(|p| p == "Bitcode") {
                    panic!("a hand-written bitcode impl: its layout isn't its fields'");
                }
                !cereal_listed(a, "not").iter().any(|p| p == "Bitcode")
            } else if path == "derive" {
                let derives = a
                    .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                    .expect("a derive's paths");
                derives.iter().any(|p| path_text(p).ends_with("bitcode::Encode"))
            } else {
                false
            }
        })
    }

    /// The names a `cereal::derived` lists under `not(..)` or `has(..)`.
    fn cereal_listed(attr: &syn::Attribute, list: &str) -> Vec<String> {
        let metas = attr
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .expect("cereal's arguments");
        metas
            .iter()
            .filter_map(|m| match m {
                Meta::List(l) if l.path.is_ident(list) => Some(l),
                _ => None,
            })
            .flat_map(|l| {
                l.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                    .expect("a cereal list's names")
                    .iter()
                    .map(path_text)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Refuses a `cfg` or `cfg_attr` on a field or a variant, which would make
    /// the layout depend on the build.
    fn fixed(attrs: &[syn::Attribute], what: &str) {
        if attrs.iter().any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr")) {
            panic!("{what} carries a cfg: the wire's layout would vary with the build");
        }
    }

    /// Whether bitcode skips the field. Any other bitcode attribute is refused
    /// until the schema records what it does.
    fn skipped(attrs: &[syn::Attribute], what: &str) -> bool {
        attrs.iter().filter(|a| a.path().is_ident("bitcode")).any(|a| match words(a).as_slice() {
            [word] if word == "skip" => true,
            other => panic!("{what}: bitcode attribute {other:?} isn't one the schema records"),
        })
    }

    /// Each field bitcode encodes, in order, by name or by position.
    fn fields(fields: &syn::Fields, owner: &str) -> Vec<Value> {
        fields
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let name = f.ident.as_ref().map_or_else(|| i.to_string(), ToString::to_string);
                let what = format!("{owner}.{name}");
                fixed(&f.attrs, &what);
                (!skipped(&f.attrs, &what)).then(|| json!({"name": name, "type": render(&f.ty)}))
            })
            .collect()
    }

    /// The generic parameters in order: a type's name, a lifetime, a const.
    fn generics(generics: &syn::Generics) -> Vec<String> {
        generics
            .params
            .iter()
            .map(|p| match p {
                syn::GenericParam::Type(t) => t.ident.to_string(),
                syn::GenericParam::Lifetime(l) => format!("'{}", l.lifetime.ident),
                syn::GenericParam::Const(c) => format!("const {}: {}", c.ident, render(&c.ty)),
            })
            .collect()
    }

    /// A type as text in one spacing, whatever the source's whitespace.
    fn render(ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(p) if p.qself.is_none() => path_text(&p.path),
            syn::Type::Array(a) => format!("[{}; {}]", render(&a.elem), length(&a.len)),
            syn::Type::Slice(s) => format!("[{}]", render(&s.elem)),
            syn::Type::Tuple(t) => {
                let elems: Vec<String> = t.elems.iter().map(render).collect();
                match elems.as_slice() {
                    [one] => format!("({one},)"),
                    _ => format!("({})", elems.join(", ")),
                }
            }
            syn::Type::Reference(r) => {
                let lifetime = r.lifetime.as_ref().map_or_else(String::new, |l| format!("'{} ", l.ident));
                let mutability = if r.mutability.is_some() { "mut " } else { "" };
                format!("&{lifetime}{mutability}{}", render(&r.elem))
            }
            syn::Type::Paren(p) => render(&p.elem),
            syn::Type::Group(g) => render(&g.elem),
            _ => panic!("a field type the schema can't describe: a qualified path, pointer, function, trait, or macro"),
        }
    }

    /// A path as `a::b<C, D>`.
    fn path_text(path: &syn::Path) -> String {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|s| match &s.arguments {
                syn::PathArguments::None => s.ident.to_string(),
                syn::PathArguments::AngleBracketed(a) => {
                    let args: Vec<String> = a
                        .args
                        .iter()
                        .map(|g| match g {
                            syn::GenericArgument::Type(t) => render(t),
                            syn::GenericArgument::Lifetime(l) => format!("'{}", l.ident),
                            syn::GenericArgument::Const(e) => length(e),
                            _ => panic!("{}: an associated type or a bound in a field's type", s.ident),
                        })
                        .collect();
                    format!("{}<{}>", s.ident, args.join(", "))
                }
                syn::PathArguments::Parenthesized(_) => panic!("{}: a function trait in a field's type", s.ident),
            })
            .collect();
        let lead = if path.leading_colon.is_some() { "::" } else { "" };
        format!("{lead}{}", segments.join("::"))
    }

    /// An array's length or a const argument: a literal integer alone, so a
    /// constant's value can't change the layout without changing the schema.
    fn length(expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(int), .. }) => int.base10_digits().to_string(),
            _ => panic!("a length the schema can't record: write it as a literal integer"),
        }
    }
}

#[test]
fn the_schema_records_every_public_wire_type_as_the_source_declares_it() {
    let generated = schema::generate();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(schema::FILE);
    if std::env::var("BLESS").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().expect("the schema's directory")).expect("created");
        std::fs::write(&path, schema::pretty(&generated)).expect("the schema written");
    }
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}; BLESS=1 writes it", schema::FILE));
    let file: serde_json::Value = serde_json::from_str(&text).expect("the schema's JSON");
    assert_eq!(
        schema::canonical(&file),
        schema::canonical(&generated),
        "a wire type changed; a deliberate change re-blesses {} and SCHEMA_ID",
        schema::FILE,
    );
}

#[test]
fn the_schema_id_is_the_hash_of_the_schema_s_canonical_form() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(schema::FILE);
    let text = std::fs::read_to_string(&path).expect("the schema");
    let id = schema::id(&text);
    // A checkout's line endings, and the file's spacing and key order, change
    // its bytes but never its ID.
    assert_eq!(schema::id(&text.replace('\n', "\r\n")), id);
    let file: serde_json::Value = serde_json::from_str(&text).expect("the schema's JSON");
    assert_eq!(schema::id(&serde_json::to_string(&file).expect("compact")), id);
    let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(SCHEMA_ID, id, "the checked-in schema's ID is {hex}");
}
