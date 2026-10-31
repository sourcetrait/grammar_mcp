use crate::*;

/// Parameters for `interface remote channel listen`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteListenParams {
    /// The remote whose peer to listen for.
    pub alias: String,
    /// The `ip` or `ip:port` to bind, in place of the entry's.
    pub on: Option<String>,
}

impl ChannelFace {
    /// `interface remote channel listen`: bind for a peer, blocking on the bind.
    pub(crate) async fn remote_listen(
        &self,
        nonce: datum::Nonce,
        p: RemoteListenParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        if let Err(error) = self.remote_gate().await {
            return error_to_call_result(error, nonce);
        }
        let listen = RemoteListenRequest {
            alias: p.alias,
            on: p.on,
        };
        self.remote_answer(nonce, listen).await
    }
}
