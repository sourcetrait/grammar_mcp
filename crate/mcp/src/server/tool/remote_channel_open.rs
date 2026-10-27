use crate::*;

/// Parameters for `interface remote open`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteChannelOpenParams {
    /// The `[[remote]]` alias to open, from `.grammar/mcp/remotes.toml`.
    pub alias: String,
}

impl NuSh {
    /// `interface remote open`: open a configured link, blocking on its network step.
    pub(crate) async fn remote_open(
        &self,
        nonce: datum::Nonce,
        p: RemoteChannelOpenParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
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
