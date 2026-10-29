//! grammard-probe's entry: its command line parsed and the command run.
use crate::*;

/// Parses the probe's command line, runs it, and exits by its status.
pub fn run_main() -> process::ExitCode {
    let cli = ProbeCli::parse();
    match run(cli.command) {
        Ok(()) => process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("grammard-probe: {e}");
            process::ExitCode::from(e.exit_code())
        }
    }
}

/// Runs one command.
fn run(command: ProbeCmd) -> ProbeResult<()> {
    match command {
        ProbeCmd::Inventory { out } => write_json(&out, &inventory()),
        ProbeCmd::Run { case, candidate_set, fixture, out } => {
            run_case(&case, &candidate_set, &fixture, &out)
        }
    }
}
