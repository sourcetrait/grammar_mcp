use crate::*;

/// Parameters for `interface remote channel set`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteSetParams {
    /// The remote to make or change.
    pub alias: String,
    /// The `ip:port` a connect dials by default.
    pub address: Option<String>,
    /// The `ip` or `ip:port` a listen binds by default.
    pub listen: Option<String>,
    /// The peer's certificate, PEM.
    pub cert: Option<String>,
}

impl ChannelFace {
    /// `interface remote channel set`: make or change an entry, answering its row.
    pub(crate) async fn remote_set(
        &self,
        nonce: datum::Nonce,
        p: RemoteSetParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let set = RemoteSetRequest {
            alias: p.alias,
            address: p.address,
            listen: p.listen,
            cert: p.cert,
        };
        self.remote_answer(nonce, set).await
    }

    /// `interface remote channel unset`: remove a remote's entry; a live link stays.
    pub(crate) async fn remote_unset(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.remote_answer(nonce, RemoteUnsetRequest { alias: p.alias }).await
    }
}
