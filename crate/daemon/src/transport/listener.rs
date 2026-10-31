//! The engine socket's own listener: bound by its worker, private to its UID.
use crate::*;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};

/// The engine socket's mode: its owner's alone.
const SOCKET_MODE: u32 = 0o600;

/// The socket directory's mode.
const DIR_MODE: u32 = 0o700;

/// The extension of the socket's own lock beside it.
const SOCKET_LOCK_EXTENSION: &str = "lock";

/// The engine socket bound at its path, removed at its drop while still its own.
#[derive(Debug)]
pub(crate) struct EngineListener {
    listener: Option<std::os::unix::net::UnixListener>,
    path: PathBuf,
    /// The bound socket's device and inode, so the drop removes this socket alone.
    bound: (u64, u64),
    /// The socket path's own lock, held while the socket is bound.
    _held: NamespaceLock,
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
        let held = NamespaceLock::acquire(&path.with_extension(SOCKET_LOCK_EXTENSION)).map_err(|e| match e {
            DaemonError::NamespaceHeld { .. } => unavailable("another worker holds its lock".to_string()),
            other => unavailable(other.to_string()),
        })?;
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_socket() && meta.uid() == uid => match probe(path) {
                Ok(Probe::Live) => return Err(unavailable("a live listener answers on it".to_string())),
                Ok(Probe::Dead) => {
                    fs::remove_file(path).map_err(|e| unavailable(format!("a dead worker's socket stays: {e}")))?;
                }
                Err(e) => return Err(unavailable(format!("the socket there can't be probed: {e}"))),
            },
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
            _held: held,
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

/// What a connect that never waits finds at a socket's path.
enum Probe {
    /// A listener answers, or holds a full backlog.
    Live,
    /// Nothing listens: a dead worker's socket.
    Dead,
}

/// The socket at `path` probed by a nonblocking connect, never waiting on it.
fn probe(path: &Path) -> io::Result<Probe> {
    let fd = nix::sys::socket::socket(
        nix::sys::socket::AddressFamily::Unix,
        nix::sys::socket::SockType::Stream,
        nix::sys::socket::SockFlag::empty(),
        None,
    )?;
    nix::fcntl::fcntl(&fd, nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC))?;
    nix::fcntl::fcntl(&fd, nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK))?;
    let addr = nix::sys::socket::UnixAddr::new(path)?;
    match nix::sys::socket::connect(std::os::fd::AsRawFd::as_raw_fd(&fd), &addr) {
        Ok(()) | Err(nix::errno::Errno::EAGAIN | nix::errno::Errno::EINPROGRESS) => Ok(Probe::Live),
        Err(nix::errno::Errno::ECONNREFUSED) => Ok(Probe::Dead),
        Err(errno) => Err(errno.into()),
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
