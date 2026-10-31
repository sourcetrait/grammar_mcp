use crate::*;

/// What a change to the current view did.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewDeltaResult {
    /// `info`'s `signatures` for what this call brought into view, or null.
    pub revealed: Option<String>,
    /// The purview ids in view now - the keys alone.
    pub current: Vec<String>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface purview extend`: bring more purviews into the current view.
    pub(crate) async fn purview_extend(
        &self,
        nonce: datum::Nonce,
        p: PurviewExtendParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let asked = match self.resolve_purviews(p.purviews.clone()).await {
            Ok(asked) => asked,
            Err(error) => return error_to_call_result(error, nonce),
        };
        if let Some(id) = asked.unknown.first() {
            return error_to_call_result(unknown_purview(id), nonce);
        }
        let before = match self.resolve_purviews(self.current_purview.ids()).await {
            Ok(before) => before,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let ids = self.current_purview.extend(&p.purviews);
        let after = match self.resolve_purviews(ids.clone()).await {
            Ok(after) => after,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let (revealed, _left) = pattern_delta(&before.patterns, &after.patterns);
        let revealed = if revealed.is_empty() {
            None
        } else {
            match self.signatures(revealed).await {
                Ok(signatures) => Some(signatures),
                Err(error) => return error_to_call_result(error, nonce),
            }
        };
        success(
            nonce,
            &PurviewDeltaResult {
                revealed,
                current: ids,
            },
        )
    }
}
