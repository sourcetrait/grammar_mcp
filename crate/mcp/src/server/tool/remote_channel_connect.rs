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
        match self.connect_to(p).await {
            Ok(()) => success_void(nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// Dial `p.alias`'s peer at the address the call or the entry names.
    async fn connect_to(
        &self,
        p: RemoteConnectParams,
    ) -> Result<(), GrammarMcpError> {
        let entry = self.remote_to_open(&p.alias).await?;
        let addr = match &p.address {
            Some(address) => dial_address(address)
                .map_err(|reason| GrammarMcpError::RemoteInvalidParams { reason })?,
            None => entry.dial().ok_or_else(|| GrammarMcpError::RemoteInvalidParams {
                reason: format!(
                    "remote channel `{}` names no address; give `--address` or set one \
                     with `interface remote channel set`",
                    p.alias,
                ),
            })?,
        };
        let keys = link_keys(&entry)?;
        connect_remote(&self.mcp_nom, p.alias.clone(), keys, addr)
            .await
            .map_err(|reason| GrammarMcpError::RemoteOpenFailed {
                alias: p.alias,
                reason,
            })
    }
}
