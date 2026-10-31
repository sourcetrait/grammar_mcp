use crate::*;

/// Parameters for `interface remote channel close`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteChannelCloseParams {
    /// The alias of the open link to close.
    pub alias: String,
}

impl ChannelFace {
    /// `interface remote channel close`: close an open link or a waiting listener.
    pub(crate) async fn remote_close(
        &self,
        nonce: datum::Nonce,
        p: RemoteChannelCloseParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.remote_answer(nonce, RemoteCloseRequest { alias: p.alias }).await
    }
}
