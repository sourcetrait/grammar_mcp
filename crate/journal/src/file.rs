//! The journal's files: its directory, lock, segment, sidecars, and faults.
use crate::*;

/// The journal's own lock, beside its files.
const LOCK_FILE: &str = "journal.lock";

/// The directory of segments, one a start.
const SEGMENTS: &str = "segments";

/// The directory of outcomes past the inline bound, each named by its hash.
const RESULTS: &str = "results";

/// A segment's extension: NUON lines.
const SEGMENT_EXTENSION: &str = "nuonl";

/// A sidecar's extension: one NUON binary value.
const SIDECAR_EXTENSION: &str = "nuon";

/// Every directory the journal makes: its owner's alone.
const DIR_MODE: u32 = 0o700;

/// Every file the journal makes: its owner's alone.
const FILE_MODE: u32 = 0o600;

/// A point a test plan can fail: a write, a sync, a rename, a directory's sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fault {
    /// A line's write into the segment.
    LineWrite,
    /// The segment's sync after a line's write.
    LineSync,
    /// A sidecar's file made under its temporary name.
    SidecarCreate,
    /// A sidecar's bytes written.
    SidecarWrite,
    /// A sidecar's file synced.
    SidecarSync,
    /// A sidecar renamed to its hash's name.
    SidecarRename,
    /// The results directory synced after a rename.
    SidecarDirSync,
}

/// The points armed to fail, each once at its nth hit, counting from one.
#[derive(Debug, Clone, Default)]
pub struct Faults {
    armed: HashMap<Fault, u32>,
    hits: HashMap<Fault, u32>,
}

impl Faults {
    /// No point armed, the plan a product runs.
    pub fn none() -> Self {
        Self::default()
    }

    /// The plan with `at` failing at its `nth` hit.
    pub fn arm(mut self, at: Fault, nth: u32) -> Self {
        self.armed.insert(at, nth);
        self
    }

    /// One hit of `at`: an error where the plan arms this one.
    pub(crate) fn hit(&mut self, at: Fault) -> io::Result<()> {
        let hits = self.hits.entry(at).or_default();
        *hits += 1;
        match self.armed.get(&at) {
            Some(nth) if nth == hits => Err(io::Error::other(format!("a fault armed at {at:?}"))),
            _ => Ok(()),
        }
    }
}

/// The journal's directory, held by its lock while it lives.
pub(crate) struct JournalDir {
    root: PathBuf,
    _lock: nix::fcntl::Flock<fs::File>,
}

/// An opened directory, the bytes its files hold, and its next segment's number.
pub(crate) struct Opened {
    pub(crate) dir: JournalDir,
    pub(crate) used: u64,
    pub(crate) next: u64,
}

impl JournalDir {
    /// `root` and its two directories made private and locked, their bytes counted.
    pub(crate) fn open(root: &Path) -> JournalResult<Opened> {
        let unavailable = |path: &Path, reason: String| JournalError::Unavailable { path: path.to_path_buf(), reason };
        let segments = root.join(SEGMENTS);
        let results = root.join(RESULTS);
        for dir in [root, segments.as_path(), results.as_path()] {
            make_dirs(dir).map_err(|e| unavailable(dir, format!("it can't be made: {e}")))?;
            private(dir).map_err(|reason| unavailable(dir, reason))?;
        }
        let lock = take_lock(&root.join(LOCK_FILE))?;
        let (segment_bytes, highest) = scan(&segments).map_err(|e| unavailable(&segments, e.to_string()))?;
        let (result_bytes, _) = scan(&results).map_err(|e| unavailable(&results, e.to_string()))?;
        Ok(Opened {
            dir: Self { root: root.to_path_buf(), _lock: lock },
            used: segment_bytes + result_bytes,
            next: highest + 1,
        })
    }

    /// The directory of segments.
    pub(crate) fn segments(&self) -> PathBuf {
        self.root.join(SEGMENTS)
    }

    /// The directory of sidecars.
    pub(crate) fn results(&self) -> PathBuf {
        self.root.join(RESULTS)
    }

    /// The journal's root.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

/// `path` and its missing parents made 0700, each synced into its parent.
fn make_dirs(path: &Path) -> io::Result<()> {
    let mut missing = Vec::new();
    let mut at = path;
    while matches!(fs::symlink_metadata(at), Err(e) if e.kind() == io::ErrorKind::NotFound) {
        missing.push(at.to_path_buf());
        match at.parent() {
            Some(parent) => at = parent,
            None => break,
        }
    }
    for dir in missing.into_iter().rev() {
        match fs::DirBuilder::new().mode(DIR_MODE).create(&dir) {
            Ok(()) => sync_dir(dir.parent().unwrap_or(Path::new("/")))?,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Why `dir` isn't a plain directory its owner alone can open, if it isn't.
fn private(dir: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(dir).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err("it is no plain directory".to_string());
    }
    let uid = nix::unistd::geteuid().as_raw();
    if meta.uid() != uid {
        return Err(format!("UID {} owns it, not {uid}", meta.uid()));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(format!("its mode {:o} opens it past 0700", meta.mode() & 0o777));
    }
    Ok(())
}

/// The journal's lock taken without waiting, refused while anyone holds it.
fn take_lock(path: &Path) -> JournalResult<nix::fcntl::Flock<fs::File>> {
    let unavailable = |reason: String| JournalError::Unavailable { path: path.to_path_buf(), reason };
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(FILE_MODE)
        .custom_flags((nix::fcntl::OFlag::O_NOFOLLOW | nix::fcntl::OFlag::O_NONBLOCK).bits())
        .open(path)
        .map_err(|e| unavailable(format!("it does not open as a plain file: {e}")))?;
    let meta = file.metadata().map_err(|e| unavailable(e.to_string()))?;
    if !meta.file_type().is_file() {
        return Err(unavailable("it is not a regular file".to_string()));
    }
    let uid = nix::unistd::geteuid().as_raw();
    if meta.uid() != uid {
        return Err(unavailable(format!("UID {} owns it, not {uid}", meta.uid())));
    }
    match nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock) {
        Ok(lock) => Ok(lock),
        Err((_, nix::errno::Errno::EWOULDBLOCK)) => Err(JournalError::Held { path: path.to_path_buf() }),
        Err((_, errno)) => Err(unavailable(errno.to_string())),
    }
}

/// The bytes of every regular file in `dir`, and the highest segment number there.
fn scan(dir: &Path) -> io::Result<(u64, u64)> {
    let mut bytes = 0u64;
    let mut highest = 0u64;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            bytes = bytes.saturating_add(meta.len());
        }
        if let Some(number) = entry.file_name().to_str().and_then(segment_number) {
            highest = highest.max(number);
        }
    }
    Ok((bytes, highest))
}

/// A segment's number from its file's name, `<n>.nuonl`, `n` in plain decimal.
fn segment_number(name: &str) -> Option<u64> {
    let stem = name.strip_suffix(SEGMENT_EXTENSION)?.strip_suffix('.')?;
    let plain = !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()) && (stem == "0" || !stem.starts_with('0'));
    if plain { stem.parse().ok() } else { None }
}

/// `dir`'s entries made durable: the directory itself synced.
fn sync_dir(dir: &Path) -> io::Result<()> {
    fs::File::open(dir)?.sync_all()
}

/// The segment this start writes, made where no segment was.
pub(crate) struct Segment {
    path: PathBuf,
    file: fs::File,
    len: u64,
    next_seq: u64,
}

impl Segment {
    /// Segment `number` made in `dir`, refused where anything has its name.
    pub(crate) fn create(dir: &Path, number: u64) -> io::Result<Self> {
        let path = dir.join(format!("{number}.{SEGMENT_EXTENSION}"));
        let file = fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .mode(FILE_MODE)
            .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
            .open(&path)?;
        sync_dir(dir)?;
        Ok(Self { path, file, len: 0, next_seq: 0 })
    }

    /// The segment's file.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// `entry` written as the next line and synced: the line's offset and length.
    pub(crate) fn append(&mut self, entry: Entry, faults: &mut Faults) -> io::Result<(u64, u64)> {
        let text = Line::new(self.next_seq, entry)?.text()?;
        faults.hit(Fault::LineWrite)?;
        self.file.write_all(text.as_bytes())?;
        faults.hit(Fault::LineSync)?;
        self.file.sync_data()?;
        let (offset, len) = (self.len, text.len() as u64);
        self.len += len;
        self.next_seq += 1;
        Ok((offset, len))
    }

    /// The line at `offset`, `len` bytes, read back and its checksum held.
    pub(crate) fn read(&self, offset: u64, len: u64) -> io::Result<Line> {
        let mut bytes = vec![0; usize::try_from(len).map_err(io::Error::other)?];
        self.file.read_exact_at(&mut bytes, offset)?;
        Line::read(std::str::from_utf8(&bytes).map_err(invalid)?)
    }
}

/// `bytes` kept under their hash, synced and named: the hash, and the bytes added.
pub(crate) fn put_sidecar(results: &Path, bytes: &[u8], faults: &mut Faults) -> io::Result<(String, u64)> {
    let sha256 = hex(&Sha256::digest(bytes));
    let path = results.join(format!("{sha256}.{SIDECAR_EXTENSION}"));
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_file() => return Ok((sha256, 0)),
        Ok(_) => return Err(io::Error::other(format!("something other than a file holds {}", path.display()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let staged = results.join(format!(".{sha256}.tmp"));
    match fs::symlink_metadata(&staged) {
        Ok(meta) if meta.is_file() => fs::remove_file(&staged)?,
        Ok(_) => return Err(io::Error::other(format!("something other than a file holds {}", staged.display()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let text = format!("{}\n", NuVal::Binary(bytes.to_vec()).to_nuon(NuonStyle::Default));
    faults.hit(Fault::SidecarCreate)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
        .open(&staged)?;
    faults.hit(Fault::SidecarWrite)?;
    file.write_all(text.as_bytes())?;
    faults.hit(Fault::SidecarSync)?;
    file.sync_data()?;
    faults.hit(Fault::SidecarRename)?;
    fs::rename(&staged, &path)?;
    faults.hit(Fault::SidecarDirSync)?;
    sync_dir(results)?;
    Ok((sha256, text.len() as u64))
}

/// A sidecar's bytes read back, refused unless they hash to its name.
pub(crate) fn read_sidecar(results: &Path, sha256: &str) -> io::Result<Vec<u8>> {
    let path = results.join(format!("{sha256}.{SIDECAR_EXTENSION}"));
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags((nix::fcntl::OFlag::O_NOFOLLOW | nix::fcntl::OFlag::O_NONBLOCK).bits())
        .open(&path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid(format!("{} is no regular file", path.display())));
    }
    let text = io::read_to_string(file)?;
    let NuVal::Binary(bytes) = NuVal::from_nuon(text.trim_end_matches('\n')).map_err(invalid)? else {
        return Err(invalid(format!("{} holds no binary value", path.display())));
    };
    if hex(&Sha256::digest(&bytes)) != sha256 {
        return Err(invalid(format!("{}'s bytes don't hash to its name", path.display())));
    }
    Ok(bytes)
}
