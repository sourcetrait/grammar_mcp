//! grammard's local CLI: a declared stub until the runtime owners exist.
use crate::*;

/// Runs one tool call in process; this build implements none, so it refuses.
pub(crate) fn run_oneshot() -> DaemonResult<()> {
    Err(DaemonError::Unimplemented {
        command: GrammardCmd::Cli.name(),
    })
}
