//! grammard's entry: its command line parsed and the command run.
use crate::*;
use nix::sys::signal::{SigSet, Signal};
use std::io::Write;

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

/// Runs one command: `serve` in the foreground, each other refused.
fn run(command: GrammardCmd) -> DaemonResult<()> {
    match command {
        GrammardCmd::Serve { id, namespace } => serve(&id, &namespace),
        GrammardCmd::Cli => run_oneshot(),
        command => Err(DaemonError::Unimplemented { command: command.name() }),
    }
}

/// `id` and `namespace`'s instance served in the foreground until a signal ends it.
fn serve(
    id: &str,
    namespace: &str,
) -> DaemonResult<()> {
    let signals = stop_signals();
    let env: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let config = InstanceConfig::load(current_uid(), id, namespace, &env)?;
    for line in &config.homes.diagnostics {
        eprintln!("grammard: {line}");
    }
    let runtime = Runtime::start(config)?;
    for line in runtime.diagnostics() {
        eprintln!("grammard: {line}");
    }
    let identity = runtime.identity();
    let mut out = std::io::stdout().lock();
    let _ = writeln!(
        out,
        "grammard: ready, {}/{} at generation {}, pid {}",
        identity.id,
        identity.namespace,
        runtime.generation().hex(),
        process::id(),
    );
    let _ = out.flush();
    drop(out);
    match signals.wait() {
        Ok(signal) => eprintln!("grammard: {signal}, stopping"),
        Err(e) => eprintln!("grammard: the stop signals can't be waited on: {e}, stopping"),
    }
    runtime.shutdown()
}

/// The signals that stop a foreground worker, blocked before any thread starts.
fn stop_signals() -> SigSet {
    let mut set = SigSet::empty();
    for signal in [Signal::SIGTERM, Signal::SIGINT, Signal::SIGHUP] {
        set.add(signal);
    }
    if let Err(e) = set.thread_block() {
        eprintln!("grammard: the stop signals can't be blocked: {e}");
    }
    set
}
