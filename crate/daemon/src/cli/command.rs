//! grammard's command line: the service, its instances, the bridge, a local CLI.

/// grammard's command line.
#[derive(clap::Parser)]
#[command(version, about = "The Grammar daemon")]
pub(crate) struct GrammardCli {
    #[command(subcommand)]
    pub(crate) command: GrammardCmd,
}

/// grammard's commands, each a namespace of its own.
#[derive(clap::Subcommand, Clone, Copy)]
pub(crate) enum GrammardCmd {
    /// Serve an instance's engine socket as a user service.
    Serve,
    /// Register and administer this user's instances.
    Instance,
    /// Bridge a controller's SSH subsystem session to its instance.
    Bridge,
    /// Run the tools in process, the local CLI.
    Cli,
}

impl GrammardCmd {
    /// The command's name, as its user typed it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Serve => "serve",
            Self::Instance => "instance",
            Self::Bridge => "bridge",
            Self::Cli => "cli",
        }
    }
}
