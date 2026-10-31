//! The engine socket's own listener: bound by its worker, private to its UID.
use crate::*;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};

/// The engine socket's mode: its owner's alone.
const SOCKET_MODE: u32 = 0o600;

/// The socket directory's mode.
const DIR_MODE: u32 = 0o700;

/// The engine socket bound at its path, removed at its drop while still its own.
#[derive(Debug)]
pub(crate) struct EngineListener {
    listener: Option<std::os::unix::net::UnixListener>,
    path: PathBuf,
    /// The bound socket's device and inode, so the drop removes this socket alone.
    bound: (u64, u64),
}

impl EngineListener {
    /// The socket at `path` bound, its directory private, a dead worker's replaced.
    pub(crate) fn bind(path: &Path) -> DaemonResult<Self> {
        let unavailable = |reason: String| DaemonError::SocketUnavailable {
            path: path.to_path_buf(),
            reason,
        };
        let uid = current_uid();
        let dir = path.parent().ok_or_else(|| unavailable("it names no directory".to_string()))?;
        match fs::DirBuilder::new().mode(DIR_MODE).create(dir) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(unavailable(format!("its directory can't be made: {e}"))),
        }
        let meta = fs::symlink_metadata(dir).map_err(|e| unavailable(e.to_string()))?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(unavailable("its directory is no plain directory".to_string()));
        }
        if meta.uid() != uid {
            return Err(unavailable(format!("UID {} owns its directory", meta.uid())));
        }
        if meta.mode() & 0o077 != 0 {
            return Err(unavailable(format!("its directory's mode {:o} opens it past 0700", meta.mode() & 0o777)));
        }
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_socket() && meta.uid() == uid => {
                fs::remove_file(path).map_err(|e| unavailable(format!("a dead worker's socket stays: {e}")))?;
            }
            Ok(_) => return Err(unavailable("something other than this UID's socket holds its path".to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(unavailable(e.to_string())),
        }
        let listener = std::os::unix::net::UnixListener::bind(path).map_err(|e| unavailable(e.to_string()))?;
        let unbind = |reason: String| {
            let _ = fs::remove_file(path);
            unavailable(reason)
        };
        fs::set_permissions(path, fs::Permissions::from_mode(SOCKET_MODE)).map_err(|e| unbind(e.to_string()))?;
        listener.set_nonblocking(true).map_err(|e| unbind(e.to_string()))?;
        let meta = fs::symlink_metadata(path).map_err(|e| unbind(e.to_string()))?;
        Ok(Self {
            listener: Some(listener),
            path: path.to_path_buf(),
            bound: (meta.dev(), meta.ino()),
        })
    }

    /// The bound listener, for the accept task, once.
    pub(crate) fn take(&mut self) -> Option<std::os::unix::net::UnixListener> {
        self.listener.take()
    }

    /// The socket's path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for EngineListener {
    fn drop(&mut self) {
        if let Ok(meta) = fs::symlink_metadata(&self.path)
            && (meta.dev(), meta.ino()) == self.bound
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}
