//! An instance's names, its id and its namespace, each held to one form.
use crate::*;

/// The most bytes an id or a namespace holds.
const NAME_MAX: usize = 64;

/// An instance's id or namespace, `[A-Za-z0-9][A-Za-z0-9._-]{0,63}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceName(String);

impl InstanceName {
    /// `text` as the `field` it names, refused where it isn't a name.
    pub fn parse(
        field: &'static str,
        text: &str,
    ) -> DaemonResult<Self> {
        let bytes = text.as_bytes();
        let valid = !bytes.is_empty()
            && bytes.len() <= NAME_MAX
            && bytes[0].is_ascii_alphanumeric()
            && bytes.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
        if valid {
            Ok(Self(text.to_string()))
        } else {
            Err(DaemonError::NameInvalid {
                field,
                name: text.to_string(),
            })
        }
    }

    /// The name's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstanceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
