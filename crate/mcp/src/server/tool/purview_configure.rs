use crate::*;

/// Success result of `interface purview configure`, the state after the write.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewConfigureResult {
    /// Everything this purview reveals; null when the call deleted it.
    pub signatures: Option<String>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface purview configure`: set a purview's patterns; empty deletes it.
    pub(crate) async fn purview_configure(
        &self,
        nonce: datum::Nonce,
        p: PurviewConfigureParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let configure = PurviewConfigureRequest {
            id: p.purview.clone(),
            patterns: p.namepaths,
        };
        let rows = match self.controller.request(configure).await {
            Ok(Ok(rows)) => rows,
            Ok(Err(error)) => return error_to_call_result(error, nonce),
            Err(error) => return error_to_call_result(error, nonce),
        };
        self.current_purview.retain_known(&rows);
        let signatures = if rows.iter().any(|row| row.id == p.purview) {
            let view = match self.resolve_purviews(vec![p.purview]).await {
                Ok(view) => view,
                Err(error) => return error_to_call_result(error, nonce),
            };
            match self.signatures(view.patterns).await {
                Ok(signatures) => Some(signatures),
                Err(error) => return error_to_call_result(error, nonce),
            }
        } else {
            None
        };
        success(nonce, &PurviewConfigureResult { signatures })
    }
}
