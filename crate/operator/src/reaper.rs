//! The host's reap of the orphans its evals leave, and its subreaper.
#[cfg(target_os = "linux")]
use crate::*;

/// How long a zombie must persist before we treat it as ABANDONED.
#[cfg(target_os = "linux")]
const REAP_GRACE_SECS: u64 = 5;

/// Parse `(state, ppid)` out of a `/proc/<pid>/stat` line.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn parse_state_ppid(stat: &str) -> Option<(String, u32)> {
    let rparen = stat.rfind(')')?;
    let mut fields = stat[rparen + 1..].split_whitespace();
    let state = fields.next()?.to_string();
    let ppid = fields.next()?.parse().ok()?;
    Some((state, ppid))
}

/// Parse a process's START TIME (field 22) out of a stat line.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn parse_start_time(stat: &str) -> Option<u64> {
    let rparen = stat.rfind(')')?;
    stat[rparen + 1..]
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}

/// A live process's start time, or None when the pid is not running.
#[cfg(target_os = "linux")]
pub fn process_start_time(pid: u32) -> Option<u64> {
    parse_start_time(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

/// A live process's start time in microseconds, or None when it is not running.
#[cfg(target_os = "macos")]
pub fn process_start_time(pid: u32) -> Option<u64> {
    use nix::libc::{PROC_PIDTBSDINFO, c_int, proc_bsdinfo, proc_pidinfo};
    let pid = c_int::try_from(pid).ok()?;
    let size = c_int::try_from(std::mem::size_of::<proc_bsdinfo>()).ok()?;
    // SAFETY: proc_bsdinfo holds integers and byte arrays alone, so all zeros
    // is a valid value of it.
    let mut info: proc_bsdinfo = unsafe { std::mem::zeroed() };
    // SAFETY: the buffer is a proc_bsdinfo of exactly `size` bytes, and
    // proc_pidinfo writes at most `size` bytes into it.
    let written =
        unsafe { proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, (&raw mut info).cast(), size) };
    (written == size).then(|| info.pbi_start_tvsec * 1_000_000 + info.pbi_start_tvusec)
}

/// Elsewhere no process table is read, so a pin can never be proven live.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn process_start_time(_pid: u32) -> Option<u64> {
    None
}

/// Harvests children the subreaper ADOPTED that nobody is waiting on.
pub(crate) struct OrphanReaper {
    #[cfg(target_os = "linux")]
    seen: HashMap<u32, Instant>,
}

impl Default for OrphanReaper {
    fn default() -> Self {
        Self::new()
    }
}

impl OrphanReaper {
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            seen: HashMap::new(),
        }
    }

    /// One pass: observe our zombies, harvest the ones past the grace window.
    #[cfg(target_os = "linux")]
    pub(crate) fn reap(&mut self) {
        let zombies = zombie_children_of(process::id());
        self.seen.retain(|pid, _| zombies.contains(pid));
        for pid in zombies {
            let first_seen = *self.seen.entry(pid).or_insert_with(Instant::now);
            if first_seen.elapsed().as_secs() < REAP_GRACE_SECS {
                continue;
            }
            let _ = nix::sys::wait::waitpid(
                nix::unistd::Pid::from_raw(pid as i32),
                Some(nix::sys::wait::WaitPidFlag::WNOHANG),
            );
            self.seen.remove(&pid);
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn reap(&mut self) {}
}

/// Our own direct children currently in state `Z`.
#[cfg(target_os = "linux")]
fn zombie_children_of(parent: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        if let Some((state, ppid)) = parse_state_ppid(&stat)
            && state == "Z"
            && ppid == parent
        {
            out.push(pid);
        }
    }
    out
}

/// Make the host the subreaper so an eval's orphans stay reachable.
#[cfg(target_os = "linux")]
pub fn install_child_subreaper() {
    if let Err(e) = nix::sys::prctl::set_child_subreaper(true) {
        eprintln!("grammar: PR_SET_CHILD_SUBREAPER failed: {e}");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn install_child_subreaper() {}
