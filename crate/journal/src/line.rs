//! A segment's line: its entry in NUON, and the checksum it carries.
use crate::*;

/// The segments' format, which the header names, raised at any entry's change.
pub(crate) const FORMAT: u32 = 1;

/// One line of a segment: its sequence, its entry, and their checksum.
#[derive(Debug, Clone, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub(crate) struct Line {
    pub(crate) seq: u64,
    pub(crate) entry: Entry,
    pub(crate) sum: String,
}

/// A segment's header, or one of the model's moves.
#[derive(Debug, Clone, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub(crate) enum Entry {
    /// The segment's first line: its format, its worker, and its start.
    Opened {
        format: u32,
        generation: String,
        instance: String,
        pid: u32,
        at: i64,
    },
    /// An operation admitted: its ticket's nonce, its payload's hash, its epoch.
    Admitted {
        operation: String,
        nonce: String,
        hash: String,
        epoch: u64,
        at: i64,
    },
    /// An admitted operation given to the engine.
    Running { operation: String, at: i64 },
    /// An operation's cancellation asked for.
    CancelRequested { operation: String, at: i64 },
    /// An operation's terminal state and where its outcome is kept.
    Finished {
        operation: String,
        state: OperationState,
        at: i64,
        outcome: Stored,
    },
}

/// Where an outcome's bytes are kept: in its entry, or in a sidecar.
#[derive(Debug, Clone, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub(crate) enum Stored {
    Inline(Blob),
    Sidecar { sha256: String, bytes: u64 },
}

/// Bytes that NUON writes as a binary value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Blob(pub(crate) Vec<u8>);

impl ser::Serialize for Blob {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> ser::Deserialize<'de> for Blob {
    fn deserialize<D: ser::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_byte_buf(BlobVisitor)
    }
}

/// A binary value's visitor.
struct BlobVisitor;

impl<'de> ser::de::Visitor<'de> for BlobVisitor {
    type Value = Blob;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a binary value")
    }

    fn visit_bytes<E: ser::de::Error>(self, bytes: &[u8]) -> Result<Blob, E> {
        Ok(Blob(bytes.to_vec()))
    }

    fn visit_byte_buf<E: ser::de::Error>(self, bytes: Vec<u8>) -> Result<Blob, E> {
        Ok(Blob(bytes))
    }
}

impl Line {
    /// `entry` as the line at `seq`, its checksum taken.
    pub(crate) fn new(seq: u64, entry: Entry) -> io::Result<Self> {
        let canonical = nuval::to_nuval(&(seq, &entry)).map_err(io::Error::other)?;
        Ok(Self { seq, sum: sum(&canonical), entry })
    }

    /// The line's NUON text on one line, its newline last.
    pub(crate) fn text(&self) -> io::Result<String> {
        let text = nuval::to_nuon(self, NuonStyle::Default).map_err(io::Error::other)?;
        if text.contains('\n') {
            return Err(io::Error::other("a line's NUON text holds a newline"));
        }
        Ok(text + "\n")
    }

    /// A line read from its text, refused unless its checksum holds.
    pub(crate) fn read(text: &str) -> io::Result<Self> {
        let value = NuVal::from_nuon(text.trim_end_matches('\n')).map_err(invalid)?;
        let canonical = NuVal::List(vec![value["seq"].clone(), value["entry"].clone()]);
        let line: Line = nuval::from_nuval(value).map_err(invalid)?;
        if line.sum != sum(&canonical) {
            return Err(invalid("a line's checksum doesn't hold"));
        }
        Ok(line)
    }
}

/// The SHA-256 of a value's bitcode, in lowercase hex.
fn sum(value: &NuVal) -> String {
    hex(&Sha256::digest(bitcode::encode(value)))
}

/// Bytes in lowercase hex.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Data that doesn't read as the journal wrote it.
pub(crate) fn invalid(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
