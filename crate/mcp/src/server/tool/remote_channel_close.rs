use crate::*;

/// Parameters for `interface remote channel close`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteChannelCloseParams {
    /// The alias of the open link to close.
    pub alias: String,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface remote channel close`: close an open link or a waiting listener.
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
