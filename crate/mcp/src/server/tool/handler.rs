use crate::*;

impl NuSh {
    pub(crate) fn tool_router() -> mcp::ToolRouter<Self> {
        let deny = &config().deny;
        let mut router = Self::processes_router()
            + Self::kill_router()
            + Self::info_router()
            + Self::inspect_router();
        if !deny.denies(DeniableTool::Nu) {
            router += Self::nu_router();
        }
        if !deny.denies(DeniableTool::Rerun) {
            router += Self::rerun_router();
        }
        if !deny.denies(DeniableTool::Call) {
            router += Self::call_router();
        }
        if !deny.denies(DeniableTool::New) {
            router += Self::new_router();
        }
        if !deny.denies(DeniableTool::Commit) {
            router += Self::commit_router();
        }
        if !deny.denies(DeniableTool::Equip) {
            router += Self::equip_router();
        }
        if !deny.denies(DeniableTool::ChannelOpen) {
            router += Self::channel_open_router();
        }
        if !deny.denies(DeniableTool::ChannelVerified) {
            router += Self::channel_verified_router();
        }
        if !deny.denies(DeniableTool::ChannelClose) {
            router += Self::channel_close_router();
        }
        if !deny.denies(DeniableTool::ConfigChannel) {
            router += Self::config_channel_router();
        }
        if !deny.denies(DeniableTool::Purviews) {
            router += Self::purviews_router();
        }
        if !deny.denies(DeniableTool::PurviewConfigure) {
            router += Self::purview_configure_router();
        }
        if !deny.denies(DeniableTool::PurviewExtend) {
            router += Self::purview_extend_router();
        }
        if !deny.denies(DeniableTool::Purview) {
            router += Self::purview_router();
        }
        if !deny.denies(DeniableTool::RemoteChannelOpen) {
            router += Self::remote_channel_open_router();
        }
        if !deny.denies(DeniableTool::RemoteChannelClose) {
            router += Self::remote_channel_close_router();
        }
        if !deny.denies(DeniableTool::RemoteChannels) {
            router += Self::remote_channels_router();
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
                "Evaluation artifacts are cached at \
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
