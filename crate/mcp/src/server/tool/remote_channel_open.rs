use crate::*;

/// Parameters for `remote_channel_open()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct RemoteChannelOpenParams {
    /// The `[[remote]]` alias to open, from `.grammar/mcp/remotes.toml`.
    pub alias: String,
}

#[mcp::tool_router(router = remote_channel_open_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(
        description = "Open a configured remote link by alias; blocks on bind (listener) or connect (connector, 20s) and returns synchronously - void on success, an error envelope on failure. A listener then emits mcp/remote/Connected when a peer pairs; Disconnected fires on any established-link teardown."
    )]
    pub(crate) async fn remote_channel_open(
        &self,
        mcp::Parameters(p): mcp::Parameters<RemoteChannelOpenParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("remote_channel_open", &p).to_nonce();
        if !matches!(self.channel.status().phase, ChannelPhase::Verified) {
            return error_to_call_result(GrammarMcpError::RemoteChannelUnverified, nonce);
        }
        let Some(entry) = config().remotes.by_alias.get(&p.alias).cloned() else {
            return error_to_call_result(
                GrammarMcpError::RemoteInvalidParams {
                    reason: format!(
                        "no remote `{}` in .grammar/mcp/remotes.toml",
                        p.alias,
                    ),
                },
                nonce,
            );
        };
        if self
            .remote_links
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&p.alias)
        {
            return error_to_call_result(
                GrammarMcpError::RemoteAlreadyOpen {
                    alias: p.alias.clone(),
                },
                nonce,
            );
        }
        match open_remote_blocking(&self.mcp_nom, entry).await {
            Ok(()) => success_void(nonce),
            Err(reason) => error_to_call_result(
                GrammarMcpError::RemoteOpenFailed {
                    alias: p.alias,
                    reason,
                },
                nonce,
            ),
        }
    }
}
