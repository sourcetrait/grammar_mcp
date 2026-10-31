//! The serving MCP of every build, a client of its runtime.
use crate::*;

/// User's envelope schema, heading the server's instructions.
const MCP_DOC: &str = include_str!("../../../../include/human/mcp/mcp.md");

/// Why a runtime-owned operation has no answer from this client.
const UNAVAILABLE_REASON: &str = "no runtime endpoint is configured for this client";

/// The serving MCP: the four tools on no engine, the Channel on its own hub.
pub struct ClientSh {
    /// The controller of the face's systems, whose Channel commands are the client's.
    controller: Arc<McpController>,
    nonce_gen: Arc<datum::NonceGenerator>,
    /// The engine an interface command parses on: its commands, nothing else.
    interface_engine: Arc<nu::EngineState>,
    tool_router: mcp::ToolRouter<ClientSh>,
}

impl ClientSh {
    pub(crate) fn new(controller: Arc<McpController>) -> Self {
        let deny = &controller.config.deny;
        Self {
            nonce_gen: controller.face.nonce_gen.clone(),
            interface_engine: Arc::new(interface_engine_for(deny)),
            tool_router: Self::tool_router(deny),
            controller,
        }
    }

    /// The four tools the deny set leaves, `interface`'s description its commands.
    pub(crate) fn tool_router(deny: &DenySet) -> mcp::ToolRouter<Self> {
        let mut router = mcp::ToolRouter::new();
        if !deny.denies(DeniableTool::Execute) {
            router += Self::execute_router();
        }
        if !deny.denies(DeniableTool::Rerun) {
            router += Self::rerun_router();
        }
        if !deny.denies(DeniableTool::Call) {
            router += Self::call_router();
        }
        if !deny.denies(DeniableTool::Interface) {
            let mut interface = Self::interface_router();
            if let Some(route) = interface.map.get_mut(INTERFACE_TOOL) {
                route.attr.description = Some(interface_description(deny).into());
            }
            router += interface;
        }
        router
    }

    /// The answer of a runtime-owned operation this client cannot reach.
    fn unavailable(nonce: datum::Nonce) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        error_to_call_result(
            GrammarMcpError::Transport(TransportError::Unavailable {
                reason: UNAVAILABLE_REASON.to_string(),
            }),
            nonce,
        )
    }

    /// A runtime-owned tool's answer: its host alias checked, then unavailable.
    fn runtime_owned(nonce: datum::Nonce, host: &str) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match resolve_host(host) {
            Ok(_) => Self::unavailable(nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// Run a parsed command: the face's systems' here, the engine's the runtime's.
    async fn run_interface(
        &self,
        nonce: datum::Nonce,
        command: InterfaceCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let face = &self.controller.face;
        match command {
            InterfaceCommand::Channel(command) => face.run_channel(nonce, command).await,
            InterfaceCommand::Peer(command) => face.run_peer(nonce, command).await,
            InterfaceCommand::Engine(_) => Self::unavailable(nonce),
        }
    }
}

#[mcp::tool_router(router = execute_router, vis = "pub(crate)")]
impl ClientSh {
    #[allow(clippy::doc_overindented_list_items)]
    #[doc = include_str!("../../../../include/human/mcp/execute.md")]
    #[mcp::tool(name = "execute")]
    pub(crate) async fn execute(
        &self,
        mcp::Parameters(p): mcp::Parameters<ExecuteParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = mint_nonce(&self.nonce_gen, "execute", &p).to_nonce();
        Self::runtime_owned(nonce, &p.host)
    }
}

#[mcp::tool_router(router = rerun_router, vis = "pub(crate)")]
impl ClientSh {
    #[mcp::tool(description = RERUN_DESCRIPTION)]
    pub(crate) async fn rerun(
        &self,
        mcp::Parameters(p): mcp::Parameters<RerunParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = mint_nonce(&self.nonce_gen, "rerun", &p).to_nonce();
        Self::runtime_owned(nonce, &p.host)
    }
}

#[mcp::tool_router(router = call_router, vis = "pub(crate)")]
impl ClientSh {
    #[doc = include_str!("../../../../include/mcp/tool/call.md")]
    #[mcp::tool(description = CALL_DESCRIPTION)]
    pub(crate) async fn call(
        &self,
        mcp::Parameters(p): mcp::Parameters<CallParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = mint_nonce(&self.nonce_gen, "call", &p).to_nonce();
        Self::runtime_owned(nonce, &p.host)
    }
}

#[mcp::tool_router(router = interface_router, vis = "pub(crate)")]
impl ClientSh {
    #[mcp::tool(description = INTERFACE_DESCRIPTION)]
    pub(crate) async fn interface(
        &self,
        mcp::Parameters(p): mcp::Parameters<InterfaceParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = mint_nonce(&self.nonce_gen, INTERFACE_TOOL, &p).to_nonce();
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

#[mcp::tool_handler(router = self.tool_router)]
impl mcp::ServerHandler for ClientSh {
    fn get_info(&self) -> mcp::ServerConfig {
        let id = &self.controller.config.id;
        let namespace = &self.controller.config.namespace;
        let title = if namespace == "default" {
            "grammar".to_string()
        } else {
            format!("grammar ({namespace})")
        };
        mcp::ServerConfig::new(mcp::ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                mcp::Implementation::new(consts::GRAMMAR, env!("CARGO_PKG_VERSION"))
                    .with_title(title),
            )
            .with_instructions(format!(
                "{MCP_DOC}\n\
                 This server's namespace: id `{id}`, namespace `{namespace}`. It reaches \
                 its runtime as a client; with no runtime endpoint configured, each \
                 runtime-owned tool and interface command answers `daemon::unavailable`.",
            ))
    }
}

/// Serve MCP over stdio as a client until the agent leaves; no engine starts.
pub(crate) async fn serve_client(config: Arc<Config>) -> Result<(), String> {
    make_scratch_dirs()?;
    ensure_cert_profile();
    let controller = McpController::client(config).await?;
    controller.sweep_signals();
    let server = ClientSh::new(controller.clone());
    let service = server
        .serve(mcp::stdio())
        .await
        .map_err(|e| format!("serve stdio: {e}"))?;
    let served = service.waiting().await;
    controller.shutdown().await;
    served
        .map(|_| ())
        .map_err(|e| format!("the stdio service: {e}"))
}
