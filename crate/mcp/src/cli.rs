use crate::*;

/// The host binary's CLI surface; a bare invocation serves MCP over stdio.
#[derive(Default, clap::Parser)]
#[command(version, about = "Grammar MCP")]
pub struct Cli {
    /// Path to a grammar_mcp.toml carrying the non-argument settings.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Agent identity owning the state namespace; defaults to the invoking user.
    #[arg(long, default_value_t = default_id())]
    pub id: String,
    /// State namespace within the id; defaults to `default`, or `test` under --test.
    #[arg(long)]
    pub namespace: Option<String>,
    /// Test channel: the namespace defaults to test; the watchdog follows the channel.
    #[arg(long)]
    pub test: bool,
    /// Agent working directory, exported to bodies as $env.EQUIP_WORK_DIR.
    #[arg(long)]
    pub workdir: Option<PathBuf>,
    /// Comma-separated tools, or interface commands by their words, to withhold.
    #[arg(long, value_delimiter = ',', value_parser = parse_deniable)]
    pub deny: Vec<DeniableTool>,
    #[command(subcommand)]
    pub command: Option<CliCmd>,
    #[arg(skip)]
    pub env: Option<CliEnv>,
}

/// Environment overrides
#[derive(Default, Debug, Clone)]
pub struct CliEnv {
    pub xdg_cache_home: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    pub xdg_state_home: Option<PathBuf>,
    pub vars: Option<HashMap<String, String>>,
}

/// The host's subcommands; absent means serve MCP over stdio.
#[derive(clap::Subcommand)]
pub enum CliCmd {
    /// One-shot CLI over the tool surface; prints the tool's NUON envelope.
    Cli {
        #[command(subcommand)]
        tool: CliTool,
    },
}

/// One-shot mirrors of the MCP tools, for an operator with no agent.
#[derive(clap::Subcommand)]
pub enum CliTool {
    /// Invoke a committed rig function.
    Call {
        /// The function's namepath: rig:module/path:function.
        namepath: String,
        /// Args as a NUON record (default {}).
        args: Option<String>,
        /// Per-call timeout in milliseconds (default 120000).
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// Execute a Nu definition against NUON data; single-shot, state dies with it.
    Nu {
        /// One `def execute` definition.
        definition: String,
        /// The definition's data as NUON text; `null` for `data: nothing`.
        data: String,
        /// The call's timeout as a Nu duration (default 120sec).
        #[arg(long)]
        timeout: Option<String>,
    },
    /// Re-evaluate an earlier nu call's definition with fresh data.
    Rerun {
        /// The nonce of the earlier nu call.
        nonce: String,
        /// Fresh data as NUON text.
        data: String,
        /// The call's timeout as a Nu duration (default 120sec).
        #[arg(long)]
        timeout: Option<String>,
    },
    /// Run one interface command, such as `interface equip commit acme/geo`.
    Interface {
        /// The command, its arguments Nu literals.
        command: String,
    },
}
