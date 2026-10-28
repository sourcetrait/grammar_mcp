//! The host's own scratch directory: one private directory a process.
use crate::*;
use std::os::unix::fs::PermissionsExt;

/// Where a scratch directory is made, memory-backed where the host has it.
const SHM_BASE: &str = "/dev/shm";

/// A scratch directory's name prefix, which the sweep matches.
const SHM_PREFIX: &str = "grammar-mcp.";

/// The marker naming a scratch directory's owner: its pid and its start time.
const OWNER_FILE: &str = "owner";

/// A scratch directory's mode: its owner's alone.
const SHM_MODE: u32 = 0o700;

/// This process's scratch directory, made once at startup.
static SHM_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Make this process's scratch directory, a dead process's swept first.
pub(crate) fn make_shm_dir() -> Result<PathBuf, String> {
    if let Some(dir) = SHM_DIR.get() {
        return Ok(dir.clone());
    }
    let base = shm_base();
    sweep_stale(&base);
    let dir = tempfile::Builder::new()
        .prefix(SHM_PREFIX)
        .permissions(fs::Permissions::from_mode(SHM_MODE))
        .tempdir_in(&base)
        .map_err(|e| format!("make a scratch directory in {}: {e}", base.display()))?
        .keep();
    let pid = process::id();
    if let Some(start) = process_start_time(pid) {
        let owner = dir.join(OWNER_FILE);
        fs::write(&owner, format!("{pid} {start}\n"))
            .map_err(|e| format!("write {}: {e}", owner.display()))?;
    }
    Ok(SHM_DIR.get_or_init(|| dir).clone())
}

/// This process's scratch directory, once `make_shm_dir` has made it.
pub(crate) fn shm_dir() -> Option<&'static Path> {
    SHM_DIR.get().map(PathBuf::as_path)
}

/// Remove this process's scratch directory at its ordinary end.
pub(crate) fn remove_shm_dir() {
    let Some(dir) = SHM_DIR.get() else {
        return;
    };
    if let Err(e) = fs::remove_dir_all(dir)
        && e.kind() != io::ErrorKind::NotFound
    {
        eprintln!("grammar: remove {}: {e}", dir.display());
    }
}

/// `/dev/shm` where the host has it, else the system's temporary directory.
fn shm_base() -> PathBuf {
    let shm = Path::new(SHM_BASE);
    if shm.is_dir() {
        shm.to_path_buf()
    } else {
        std::env::temp_dir()
    }
}

/// Remove each scratch directory whose owning process is gone.
fn sweep_stale(base: &Path) {
    let Ok(entries) = fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_str().is_some_and(|name| name.starts_with(SHM_PREFIX)) {
            continue;
        }
        let dir = entry.path();
        let Some((pid, start)) = read_owner(&dir) else {
            continue;
        };
        if process_start_time(pid) != Some(start) {
            let _ = fs::remove_dir_all(&dir);
        }
    }
}

/// A scratch directory's owner, none when its marker is absent or unreadable.
fn read_owner(dir: &Path) -> Option<(u32, u64)> {
    let text = fs::read_to_string(dir.join(OWNER_FILE)).ok()?;
    let mut parts = text.split_whitespace();
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}
