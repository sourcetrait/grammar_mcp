use crate::*;

/// Success result of `interface purview set`.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewSetResult {
    /// `info`'s `signatures` for what this call brought into view, or null.
    pub revealed: Option<String>,
}

/// Parameters for `interface purview set`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct PurviewParams {
    /// The purview ids to put in view, replacing the current set.
    pub purviews: Vec<String>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface purview set`: replace the current view with these purviews.
    pub(crate) async fn purview_set(
        &self,
        nonce: datum::Nonce,
        p: PurviewParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let want: Vec<String> = p
            .purviews
            .iter()
            .map(|id| purview_ref(id).unwrap_or(id).to_string())
            .collect();
        let wanted = match self.resolve_purviews(want.clone()).await {
            Ok(wanted) => wanted,
            Err(error) => return error_to_call_result(error, nonce),
        };
        if let Some(id) = wanted.unknown.first() {
            return error_to_call_result(unknown_purview(id), nonce);
        }
        let before = match self.resolve_purviews(self.current_purview.ids()).await {
            Ok(before) => before,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let ids = self.current_purview.set(&want);
        let after = if ids == want {
            wanted
        } else {
            match self.resolve_purviews(ids).await {
                Ok(after) => after,
                Err(error) => return error_to_call_result(error, nonce),
            }
        };
        let (revealed, _hidden) = pattern_delta(&before.patterns, &after.patterns);
        let block = if revealed.is_empty() {
            None
        } else {
            match self.signatures(revealed).await {
                Ok(signatures) => Some(signatures),
                Err(error) => return error_to_call_result(error, nonce),
            }
        };
        success(nonce, &PurviewSetResult { revealed: block })
    }
}
