use crate::*;

/// Parameters for `interface remote channel connect`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteConnectParams {
    /// The remote whose peer to dial.
    pub alias: String,
    /// The `ip:port` to dial, in place of the entry's.
    pub address: Option<String>,
}

impl ChannelFace {
    /// `interface remote channel connect`: dial a peer, blocking on the connect.
    pub(crate) async fn remote_connect(
        &self,
        nonce: datum::Nonce,
        p: RemoteConnectParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        if let Err(error) = self.remote_gate().await {
            return error_to_call_result(error, nonce);
        }
        let connect = RemoteConnectRequest {
            alias: p.alias,
            address: p.address,
        };
        self.remote_answer(nonce, connect).await
    }
}
