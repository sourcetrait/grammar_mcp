use crate::*;

/// Parameters for `remote_channel_close()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct RemoteChannelCloseParams {
    /// The alias of the open link to close.
    pub alias: String,
}

#[mcp::tool_router(router = remote_channel_close_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = "Close an open mTLS link to a remote grammar host.")]
    pub(crate) async fn remote_channel_close(
        &self,
        mcp::Parameters(p): mcp::Parameters<RemoteChannelCloseParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("remote_channel_close", &p).to_nonce();
        let entry = self
            .remote_links
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&p.alias);
        match entry {
            Some(entry) => {
                entry.handle.cancel();
                success_void(nonce)
            }
            None if close_listener(&p.alias).await => success_void(nonce),
            None => error_to_call_result(GrammarMcpError::RemoteNotOpen { alias: p.alias }, nonce),
        }
    }
}
