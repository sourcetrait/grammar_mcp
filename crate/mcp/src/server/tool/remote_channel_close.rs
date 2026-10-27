use crate::*;

/// Parameters for `interface remote close`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteChannelCloseParams {
    /// The alias of the open link to close.
    pub alias: String,
}

impl NuSh {
    /// `interface remote close`: close an open link, or a listener still waiting.
    pub(crate) async fn remote_close(
        &self,
        nonce: datum::Nonce,
        p: RemoteChannelCloseParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
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
