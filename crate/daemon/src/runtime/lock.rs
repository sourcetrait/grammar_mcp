//! The namespace lock: one evaluator a namespace, held from a start to its end.
use crate::*;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

/// The lock's file in the namespace's data home, the MCP host's own.
pub(crate) const LOCK_FILE: &str = "host.lock";

/// The most bytes of a holder's record a contention report shows.
const HOLDER_MAX: u64 = 1024;

/// What a worker writes into the lock it holds: whose namespace, which start.
#[derive(Debug, Clone, PartialEq, Eq, ser::Serialize)]
pub(crate) struct LockRecord {
    pub(crate) holder: &'static str,
    pub(crate) uid: u32,
    pub(crate) id: String,
    pub(crate) namespace: String,
    pub(crate) pid: u32,
    pub(crate) start: u64,
    pub(crate) boot: Option<String>,
    pub(crate) generation: String,
}

/// The namespace's lock, held while it lives; its drop releases the lock.
#[derive(Debug)]
pub(crate) struct NamespaceLock {
    file: nix::fcntl::Flock<fs::File>,
    path: PathBuf,
}

impl NamespaceLock {
    /// The lock at `path` taken without waiting, refused while anyone holds it.
    pub(crate) fn acquire(path: &Path) -> DaemonResult<Self> {
        let unavailable = |reason: String| DaemonError::LockUnavailable {
            path: path.to_path_buf(),
            reason,
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| unavailable(e.to_string()))?;
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags((nix::fcntl::OFlag::O_NOFOLLOW | nix::fcntl::OFlag::O_NONBLOCK).bits())
            .open(path)
            .map_err(|e| unavailable(format!("it does not open as a plain file: {e}")))?;
        let meta = file.metadata().map_err(|e| unavailable(e.to_string()))?;
        if !meta.file_type().is_file() {
            return Err(unavailable("it is not a regular file".to_string()));
        }
        if meta.uid() != current_uid() {
            return Err(unavailable(format!("UID {} owns it", meta.uid())));
        }
        match nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock) {
            Ok(file) => Ok(Self {
                file,
                path: path.to_path_buf(),
            }),
            Err((mut file, nix::errno::Errno::EWOULDBLOCK)) => Err(DaemonError::NamespaceHeld {
                path: path.to_path_buf(),
                holder: holder_of(&mut file),
            }),
            Err((_, errno)) => Err(unavailable(errno.to_string())),
        }
    }

    /// The holder's record written into the lock in place, its inode kept.
    pub(crate) fn record(
        &mut self,
        record: &LockRecord,
    ) -> DaemonResult<()> {
        let unavailable = |reason: String| DaemonError::LockUnavailable {
            path: self.path.clone(),
            reason,
        };
        let text = nuval::to_nuon(record, NuonStyle::Default).map_err(|e| unavailable(e.to_string()))?;
        self.file.set_len(0).map_err(|e| unavailable(e.to_string()))?;
        self.file.seek(SeekFrom::Start(0)).map_err(|e| unavailable(e.to_string()))?;
        self.file
            .write_all(format!("{text}\n").as_bytes())
            .map_err(|e| unavailable(e.to_string()))?;
        self.file.sync_data().map_err(|e| unavailable(e.to_string()))
    }
}

/// A holder's record as its lock holds it, on one line and bounded.
fn holder_of(file: &mut fs::File) -> String {
    let mut text = String::new();
    if file.seek(SeekFrom::Start(0)).is_ok() {
        let _ = Read::by_ref(file).take(HOLDER_MAX).read_to_string(&mut text);
    }
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        "a holder that wrote no record".to_string()
    } else {
        line
    }
}
