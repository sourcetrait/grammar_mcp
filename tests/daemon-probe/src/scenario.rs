//! A case's run: its case and manifests checked, its result written by status.
use crate::*;

/// The manifest schema version this probe reads.
const SCHEMA_VERSION: u64 = 1;

/// A case's result, as the qualification matrix reads it.
#[derive(ser::Serialize)]
pub(crate) struct ScenarioResult {
    /// The case run.
    case: String,
    /// `pass`, `fail`, or `unrun`.
    status: &'static str,
    /// The probe's exit status for the case.
    exit: u8,
    /// Why the case ended as it did.
    reason: String,
    /// The probe and its version.
    probe: &'static str,
    /// When the run started, in milliseconds since the Unix epoch.
    started_at_ms: u128,
    /// When the run ended, in milliseconds since the Unix epoch.
    ended_at_ms: u128,
    /// The assertions the case ran.
    assertions: Vec<&'static str>,
}

/// Runs one case; this build implements none, so a valid run ends unrun.
pub(crate) fn run_case(
    case: &str,
    candidate_set: &Path,
    fixture: &Path,
    out: &Path,
) -> ProbeResult<()> {
    let started_at_ms = now_ms();
    if !GROUPS.contains(&case) {
        return Err(ProbeError::Config { reason: format!("unknown case {case}") });
    }
    manifest(candidate_set)?;
    manifest(fixture)?;
    fs::create_dir_all(out)
        .map_err(|e| ProbeError::Io { path: out.to_path_buf(), reason: e.to_string() })?;
    let reason = format!("the {case} case is not implemented in this build");
    let unrun = ProbeError::Unrun { reason: reason.clone() };
    let result = ScenarioResult {
        case: case.to_string(),
        status: "unrun",
        exit: unrun.exit_code(),
        reason,
        probe: PROBE,
        started_at_ms,
        ended_at_ms: now_ms(),
        assertions: Vec::new(),
    };
    write_json(&out.join("result.json"), &result)?;
    Err(unrun)
}

/// Checks a manifest is a JSON object at this probe's schema version.
fn manifest(path: &Path) -> ProbeResult<()> {
    let refused = |reason: String| ProbeError::Config {
        reason: format!("{}: {reason}", path.display()),
    };
    let text = fs::read_to_string(path).map_err(|e| refused(e.to_string()))?;
    let value: json::Value = json::from_str(&text).map_err(|e| refused(e.to_string()))?;
    match value.get("schema_version").and_then(json::Value::as_u64) {
        Some(SCHEMA_VERSION) => Ok(()),
        _ => Err(refused(format!("schema_version is not {SCHEMA_VERSION}"))),
    }
}

/// Milliseconds since the Unix epoch, zero for a clock set before it.
fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}
