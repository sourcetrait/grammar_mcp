//! An instance's key, the hash that names its files apart from its UID.
use crate::*;

/// The lower-case SHA-256 hex of an instance's length-prefixed id and namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceKey(String);

impl InstanceKey {
    /// The key of `id` and `namespace`.
    pub fn of(
        id: &InstanceName,
        namespace: &InstanceName,
    ) -> Self {
        let mut hasher = Sha256::new();
        for part in [id.as_str(), namespace.as_str()] {
            let len = u32::try_from(part.len()).expect("a name holds 64 bytes at most");
            hasher.update(len.to_be_bytes());
            hasher.update(part.as_bytes());
        }
        let digest = hasher.finalize();
        Self(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    /// The key's hex text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstanceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
