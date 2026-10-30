//! grammard-probe's command line: the inventory and a case's run.
use crate::*;

/// grammard-probe's command line.
#[derive(clap::Parser)]
#[command(version, about = "The Grammar daemon's qualification probe")]
pub(crate) struct ProbeCli {
    #[command(subcommand)]
    pub(crate) command: ProbeCmd,
}

/// grammard-probe's commands.
#[derive(clap::Subcommand)]
pub(crate) enum ProbeCmd {
    /// Write the scenario groups and their assertions as NUON.
    Inventory {
        /// The new file the inventory is written to.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run one case against a candidate set and a fixture.
    Run {
        /// The case, a scenario group's name.
        #[arg(long)]
        case: String,
        /// The candidate set's NUON manifest.
        #[arg(long)]
        candidate_set: PathBuf,
        /// The fixture's NUON descriptor, its redacted identity manifest.
        #[arg(long)]
        fixture: PathBuf,
        /// The directory the case's result is written to.
        #[arg(long)]
        out: PathBuf,
    },
}
