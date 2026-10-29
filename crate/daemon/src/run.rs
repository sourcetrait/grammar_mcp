//! grammard's entry: its command line parsed and the command run.
use crate::*;

/// Parses grammard's command line, runs the command, and exits by its result.
pub fn run_main() -> process::ExitCode {
    let cli = GrammardCli::parse();
    match run(cli.command) {
        Ok(()) => process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("grammard: {e}");
            process::ExitCode::from(e.exit_code())
        }
    }
}

/// Runs one command; this build implements none, so each refuses.
fn run(command: GrammardCmd) -> DaemonResult<()> {
    match command {
        GrammardCmd::Cli => run_oneshot(),
        command => Err(DaemonError::Unimplemented { command: command.name() }),
    }
}
