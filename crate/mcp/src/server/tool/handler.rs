use crate::*;

/// User's envelope schema, heading the server's instructions.
const MCP_DOC: &str = include_str!("../../../../../include/human/mcp/mcp.md");

impl NuSh {
    pub(crate) fn tool_router() -> mcp::ToolRouter<Self> {
        let deny = &config().deny;
        let mut router = mcp::ToolRouter::new();
        if !deny.denies(DeniableTool::Nu) {
            router += Self::nu_router();
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
}

#[mcp::tool_handler(router = self.tool_router)]
impl mcp::ServerHandler for NuSh {
    fn get_info(&self) -> mcp::ServerConfig {
        let id = &config().id;
        let namespace = &config().namespace;
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
                 Evaluation artifacts are cached at \
                 $XDG_CACHE_HOME/grammar/mcp/{id}/{namespace}/{{nu,calls}}/<nonce>/{{stdout,stderr}}; \
                 a nu call's definition co-locates at \
                 $XDG_CACHE_HOME/grammar/mcp/{id}/{namespace}/nu/<nonce>/definition.nu \
                 (the rerun(nonce) handle). \
                 Installed equips live in a signed git repo at \
                 $XDG_DATA_HOME/grammar/mcp/{id}/{namespace}/equip/; signing keypair at \
                 $XDG_DATA_HOME/grammar/mcp/{id}/{namespace}/keypair/. \
                 This server's namespace: id `{id}`, namespace `{namespace}`.",
            ))
    }
}
