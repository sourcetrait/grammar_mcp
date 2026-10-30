//! The probe's outputs: NUON files written once, never over an existing file.
use crate::*;

/// Writes a value as NUON to a new file, refusing one that exists.
pub(crate) fn write_nuon(path: &Path, value: &impl ser::Serialize) -> ProbeResult<()> {
    let failed = |e: String| ProbeError::Io { path: path.to_path_buf(), reason: e };
    let text = nuval::to_nuon(value, NuonStyle::Spaces(1)).map_err(|e| failed(e.to_string()))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => ProbeError::Config {
                reason: format!("{} exists, and an output is never overwritten", path.display()),
            },
            _ => failed(e.to_string()),
        })?;
    file.write_all(format!("{text}\n").as_bytes()).map_err(|e| failed(e.to_string()))
}
