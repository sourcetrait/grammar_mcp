use crate::*;

/// Parameters for `remote_channels()` (none).
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct RemoteChannelsParams {}

/// One established remote link in the `remote_channels()` snapshot.
#[derive(Debug, ser::Serialize)]
pub(crate) struct RemoteChannelEntry {
    pub alias: String,
    pub remote_mcp_nom: String,
    pub address: String,
}

/// One bound-but-unpaired listener in the `listening` set: its alias + bind address.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ListeningEntry {
    pub remote: String,
    pub address: String,
}

/// Success result of `remote_channels()`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct RemoteChannelsResult {
    pub channels: Vec<RemoteChannelEntry>,
    pub listening: Vec<ListeningEntry>,
}

#[mcp::tool_router(router = remote_channels_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(
        description = "List remote grammar-host links: established `channels` plus bound-but-unpaired `listening` listeners."
    )]
    pub(crate) async fn remote_channels(
        &self,
        mcp::Parameters(p): mcp::Parameters<RemoteChannelsParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("remote_channels", &p).to_nonce();
        let links = self.remote_links.lock().unwrap_or_else(|e| e.into_inner());
        let mut channels: Vec<RemoteChannelEntry> = links
            .iter()
            .map(|(alias, entry)| RemoteChannelEntry {
                alias: alias.clone(),
                remote_mcp_nom: entry.handle.remote_mcp_nom.clone(),
                address: entry.addr.to_string(),
            })
            .collect();
        drop(links);
        channels.sort_by(|a, b| a.alias.cmp(&b.alias));

        let listeners = bound_listeners();
        let bound = listeners.lock().unwrap_or_else(|e| e.into_inner());
        let mut listening: Vec<ListeningEntry> = bound
            .iter()
            .map(|(alias, listener)| ListeningEntry {
                remote: alias.clone(),
                address: listener.addr.to_string(),
            })
            .collect();
        drop(bound);
        listening.sort_by(|a, b| a.remote.cmp(&b.remote));

        success(nonce, &RemoteChannelsResult { channels, listening })
    }
}
