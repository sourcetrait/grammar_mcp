//! Layered teardown of an eval's external process tree.
use crate::*;

/// Build this eval's external-child tracker.
pub(crate) fn make_tracker(cancel: Arc<AtomicBool>) -> nu::ThreadJob {
    let (sender, _rx) = std::sync::mpsc::channel::<nu::Mail>();
    nu::ThreadJob::new(
        nu::Signals::new(cancel),
        Some("grammar eval".to_string()),
        sender,
    )
}

/// SIGKILL every tracked child AND its /proc descendants.
#[cfg(target_os = "linux")]
pub(crate) fn tree_kill(tracked: &[u32]) {
    let mut victims: Vec<u32> = Vec::new();
    for &root in tracked {
        for descendant in descendants_of(root) {
            if !victims.contains(&descendant) {
                victims.push(descendant);
            }
        }
        if !victims.contains(&root) {
            victims.push(root);
        }
    }
    for pid in victims {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn tree_kill(tracked: &[u32]) {
    for &pid in tracked {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
}

/// SIGKILL the host's plugin subprocesses.
#[cfg(target_os = "linux")]
pub(crate) fn kill_plugin_subprocesses() {
    let me = std::process::id();
    let Ok(entries) = fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let (Some(lparen), Some(rparen)) = (stat.find('('), stat.rfind(')')) else {
            continue;
        };
        let comm = &stat[lparen + 1..rparen];
        let mut fields = stat[rparen + 1..].split_whitespace();
        let _state = fields.next();
        let Some(ppid) = fields.next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        if ppid == me && comm.starts_with("nu_plugin_") {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn kill_plugin_subprocesses() {}

/// Every descendant pid of `root`, read from /proc.
#[cfg(target_os = "linux")]
fn descendants_of(root: u32) -> Vec<u32> {
    let mut ppid_of: HashMap<u32, u32> = HashMap::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(rparen) = stat.rfind(')') else {
            continue;
        };
        let mut fields = stat[rparen + 1..].split_whitespace();
        let _state = fields.next();
        if let Some(ppid) = fields.next().and_then(|p| p.parse::<u32>().ok()) {
            ppid_of.insert(pid, ppid);
        }
    }
    let mut out: Vec<u32> = Vec::new();
    let mut stack = vec![root];
    while let Some(parent) = stack.pop() {
        for (&pid, &ppid) in &ppid_of {
            if ppid == parent && pid != root && !out.contains(&pid) {
                out.push(pid);
                stack.push(pid);
            }
        }
    }
    out
}
