use crate::*;

/// The tool's name on the grammar server.
pub(crate) const INTERFACE_TOOL: &str = "interface";

/// User's pretext, verbatim, opening the tool's description.
const INTERFACE_PRETEXT: &str = include_str!("../../../../../include/human/mcp/interface.md");

/// Parameters for `interface()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct InterfaceParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// One `interface` command, its arguments literals.
    pub command: String,
}

/// The tool's description: the pretext, then the commands the deny set leaves.
pub(crate) fn interface_description(deny: &DenySet) -> String {
    format!("{}\n\n{}", INTERFACE_PRETEXT.trim_end(), render_command_list(deny))
}

#[mcp::tool_router(router = interface_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = "Interfaces directly with the Grammar MCP.")]
    pub(crate) async fn interface(
        &self,
        mcp::Parameters(p): mcp::Parameters<InterfaceParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint(INTERFACE_TOOL, &p).to_nonce();
        if let Err(error) = resolve_host(&p.host) {
            return error_to_call_result(error, nonce);
        }
        match parse_command(&self.interface_engine, &p.command) {
            Ok(command) => self.run_interface(nonce, command).await,
            Err(diagnostics) => {
                error_to_call_result(GrammarMcpError::InterfaceViolations { diagnostics }, nonce)
            }
        }
    }
}

impl NuSh {
    /// Run a parsed command under the interface call's nonce.
    pub(crate) async fn run_interface(
        &self,
        nonce: datum::Nonce,
        command: InterfaceCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match command {
            InterfaceCommand::Info => self.info(nonce).await,
            InterfaceCommand::Help(p) => self.help(nonce, p).await,
            InterfaceCommand::ProcessList => self.process_list(nonce).await,
            InterfaceCommand::ProcessKill(p) => self.process_kill(nonce, p).await,
            InterfaceCommand::PurviewList => self.purview_list(nonce).await,
            InterfaceCommand::PurviewSet(p) => self.purview_set(nonce, p).await,
            InterfaceCommand::PurviewExtend(p) => self.purview_extend(nonce, p).await,
            InterfaceCommand::PurviewConfigure(p) => self.purview_configure(nonce, p).await,
            InterfaceCommand::ChannelOpen(p) => self.channel_open(nonce, p).await,
            InterfaceCommand::ChannelSync => self.channel_sync(nonce).await,
            InterfaceCommand::ChannelReady => self.channel_ready(nonce).await,
            InterfaceCommand::ChannelClose => self.channel_close(nonce).await,
            InterfaceCommand::ChannelConfig(p) => self.channel_config(nonce, p).await,
            InterfaceCommand::RemoteList => self.remote_list(nonce).await,
            InterfaceCommand::RemoteGet(p) => self.remote_get(nonce, p).await,
            InterfaceCommand::RemoteSet(p) => self.remote_set(nonce, p).await,
            InterfaceCommand::RemoteUnset(p) => self.remote_unset(nonce, p).await,
            InterfaceCommand::RemoteListen(p) => self.remote_listen(nonce, p).await,
            InterfaceCommand::RemoteConnect(p) => self.remote_connect(nonce, p).await,
            InterfaceCommand::RemoteClose(p) => self.remote_close(nonce, p).await,
            InterfaceCommand::EquipNew(p) => self.equip_new(nonce, p).await,
            InterfaceCommand::EquipInstall(p) => self.equip_install(nonce, p).await,
            InterfaceCommand::EquipCheck(p) => self.equip_check(nonce, p).await,
            InterfaceCommand::EquipUninstall(p) => self.equip_uninstall(nonce, p).await,
            InterfaceCommand::EquipScaffold(p) => self.equip_scaffold(nonce, p).await,
            InterfaceCommand::EquipCommit(p) => self.equip_commit(nonce, p).await,
        }
    }
}
