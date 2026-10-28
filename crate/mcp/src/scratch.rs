//! The host's own scratch directories: one for bulk, one transient, a process.
use crate::*;
use std::os::unix::fs::PermissionsExt;

/// A scratch directory's name prefix, which the sweep matches.
const SCRATCH_PREFIX: &str = "grammar-mcp.";

/// The marker naming a scratch directory's owner: its pid and its start time.
const OWNER_FILE: &str = "owner";

/// A scratch directory's mode: its owner's alone.
const SCRATCH_MODE: u32 = 0o700;

/// This process's scratch directories, made once at startup.
static SCRATCH: OnceLock<ScratchDirs> = OnceLock::new();

/// A process's two scratch directories.
#[derive(Debug, Clone)]
pub(crate) struct ScratchDirs {
    /// Bulk, under the temporary directory: a definition's `$env.EQUIP_TMP`.
    pub(crate) tmp: PathBuf,
    /// Small runtime files, under the runtime directory: `$env.EQUIP_TRANSIENT`.
    pub(crate) transient: PathBuf,
}

/// Make this process's scratch directories, a dead process's swept first.
pub(crate) fn make_scratch_dirs() -> Result<ScratchDirs, String> {
    if let Some(dirs) = SCRATCH.get() {
        return Ok(dirs.clone());
    }
    let tmp_base = tmp_base();
    let transient_base = runtime_base().unwrap_or_else(|| {
        eprintln!(
            "grammar: neither UENV_USR_TRANSIENT nor XDG_RUNTIME_DIR names a directory; \
             transient files go under {}",
            tmp_base.display(),
        );
        tmp_base.clone()
    });
    sweep_stale(&tmp_base);
    if transient_base != tmp_base {
        sweep_stale(&transient_base);
    }
    let tmp = make_dir(&tmp_base)?;
    let transient = make_dir(&transient_base).inspect_err(|_| {
        let _ = fs::remove_dir_all(&tmp);
    })?;
    Ok(SCRATCH.get_or_init(|| ScratchDirs { tmp, transient }).clone())
}

/// This process's bulk scratch directory, once `make_scratch_dirs` has made it.
pub(crate) fn tmp_dir() -> Option<&'static Path> {
    SCRATCH.get().map(|dirs| dirs.tmp.as_path())
}

/// This process's transient scratch directory, once made.
pub(crate) fn transient_dir() -> Option<&'static Path> {
    SCRATCH.get().map(|dirs| dirs.transient.as_path())
}

/// Remove this process's scratch directories at its ordinary end.
pub(crate) fn remove_scratch_dirs() {
    let Some(dirs) = SCRATCH.get() else {
        return;
    };
    for dir in [&dirs.tmp, &dirs.transient] {
        if let Err(e) = fs::remove_dir_all(dir)
            && e.kind() != io::ErrorKind::NotFound
        {
            eprintln!("grammar: remove {}: {e}", dir.display());
        }
    }
}

/// The temporary directory: `UENV_USR_TMP`, else `TMPDIR`, else `/tmp`.
fn tmp_base() -> PathBuf {
    home_var("UENV_USR_TMP").unwrap_or_else(|| PathBuf::from(TMP_DEFAULT))
}

/// The runtime directory: `UENV_USR_TRANSIENT`, else `XDG_RUNTIME_DIR`.
fn runtime_base() -> Option<PathBuf> {
    home_var("UENV_USR_TRANSIENT")
}

/// One scratch directory in `base`, mode 0700 from creation, its owner marked.
fn make_dir(base: &Path) -> Result<PathBuf, String> {
    let dir = tempfile::Builder::new()
        .prefix(SCRATCH_PREFIX)
        .permissions(fs::Permissions::from_mode(SCRATCH_MODE))
        .tempdir_in(base)
        .map_err(|e| format!("make a scratch directory in {}: {e}", base.display()))?
        .keep();
    let pid = process::id();
    if let Some(start) = process_start_time(pid) {
        let owner = dir.join(OWNER_FILE);
        fs::write(&owner, format!("{pid} {start}\n"))
            .map_err(|e| format!("write {}: {e}", owner.display()))?;
    }
    Ok(dir)
}

/// Remove each scratch directory whose owning process is gone.
fn sweep_stale(base: &Path) {
    let Ok(entries) = fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_str().is_some_and(|name| name.starts_with(SCRATCH_PREFIX)) {
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
