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
        match self.listen_for(p).await {
            Ok(bound) => success(
                nonce,
                &RemoteListenResult {
                    endpoint: bound.to_string(),
                },
            ),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// Bind for `p.alias`'s peer on the address the call or the entry names.
    async fn listen_for(
        &self,
        p: RemoteListenParams,
    ) -> Result<SocketAddr, GrammarMcpError> {
        let entry = self.remote_to_open(&p.alias)?;
        let bind = match &p.on {
            Some(on) => {
                bind_address(on).map_err(|reason| GrammarMcpError::RemoteInvalidParams { reason })?
            }
            None => entry.bind(),
        };
        let keys = link_keys(&entry)?;
        listen_remote(&self.mcp_nom, p.alias.clone(), keys, bind)
            .await
            .map_err(|reason| GrammarMcpError::RemoteOpenFailed {
                alias: p.alias,
                reason,
            })
    }
}
