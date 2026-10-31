use crate::*;

/// Success result of `interface purview list`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewsResult {
    /// Every configured purview, as persisted.
    pub purviews: Vec<PurviewView>,
    /// The purview ids in view right now - the keys alone.
    pub current: Vec<String>,
}

impl NuSh {
    /// `interface purview list`: every configured purview, and what is in view.
    pub(crate) async fn purview_list(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let purviews = match self.system.request(PurviewListRequest).await {
            Ok(Ok(purviews)) => purviews,
            Ok(Err(error)) => return error_to_call_result(error, nonce),
            Err(error) => return error_to_call_result(error, nonce),
        };
        success(
            nonce,
            &PurviewsResult {
                purviews,
                current: self.current_purview.ids(),
            },
        )
    }
}
