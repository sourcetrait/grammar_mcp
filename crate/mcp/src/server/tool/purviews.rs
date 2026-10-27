use crate::*;

/// Parameters for `purviews()` (none).
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct PurviewsParams {}

/// Success result of `purviews()`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewsResult {
    /// Every CONFIGURED purview, as persisted.
    pub purviews: Vec<PurviewView>,
    /// The purview ids in view right now - the KEYS alone.
    pub current: Vec<String>,
}

#[mcp::tool_router(router = purviews_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = "List every configured purview, and what is currently in view.")]
    pub(crate) async fn purviews(
        &self,
        mcp::Parameters(p): mcp::Parameters<PurviewsParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("purviews", &p).to_nonce();
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
