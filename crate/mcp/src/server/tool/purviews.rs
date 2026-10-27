use crate::*;

/// Success result of `interface purview list`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewsResult {
    /// Every CONFIGURED purview, as persisted.
    pub purviews: Vec<PurviewView>,
    /// The purview ids in view right now - the KEYS alone.
    pub current: Vec<String>,
}

impl NuSh {
    /// `interface purview list`: every configured purview, and what is in view.
    pub(crate) async fn purview_list(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let rows = match load_purviews() {
            Ok(rows) => rows,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let configured: Vec<String> = rows
            .as_ref()
            .map(|r| r.iter().map(|row| row.id.clone()).collect())
            .unwrap_or_default();
        success(
            nonce,
            &PurviewsResult {
                purviews: purview_views(&configured, rows.as_ref()),
                current: self.current_purview.ids(),
            },
        )
    }
}
