use crate::*;

/// Parameters for `purview_extend()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct PurviewExtendParams {
    /// Purview ids to add to the current view; additive.
    pub purviews: Vec<String>,
}

/// What a change to the current view did.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewDeltaResult {
    /// `info()`'s `signatures` for what this call brought INTO view, or null.
    pub revealed: Option<String>,
    /// The purview ids in view now - the KEYS alone.
    pub current: Vec<String>,
}

impl NuSh {
    /// The shared delta report for a change to the current view.
    pub(crate) async fn purview_delta(
        &self,
        nonce: datum::Nonce,
        before: Vec<String>,
        after: Vec<String>,
        ids: Vec<String>,
        rows: Option<Vec<PurviewRow>>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let (revealed, _left) = pattern_delta(
            &expand_values(&before, rows.as_ref()),
            &expand_values(&after, rows.as_ref()),
        );
        let revealed_block = if revealed.is_empty() {
            None
        } else {
            Some(render_signatures_within(&self.rig_locks, &parse_patterns(&revealed)).await)
        };
        success(
            nonce,
            &PurviewDeltaResult {
                revealed: revealed_block,
                current: ids,
            },
        )
    }
}

#[mcp::tool_router(router = purview_extend_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = "Bring more purviews into the current view.")]
    pub(crate) async fn purview_extend(
        &self,
        mcp::Parameters(p): mcp::Parameters<PurviewExtendParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("purview_extend", &p).to_nonce();
        let rows = match load_purviews() {
            Ok(rows) => rows,
            Err(error) => return error_to_call_result(error, nonce),
        };
        for id in &p.purviews {
            if !is_nameable_purview(id, rows.as_ref()) {
                return error_to_call_result(
                    GrammarMcpError::PurviewInvalidId {
                        id: id.clone(),
                        reason: "no such purview; configure it first".to_string(),
                    },
                    nonce,
                );
            }
        }
        let before = resolve_patterns(&self.current_purview.ids(), rows.as_ref());
        let ids = self.current_purview.extend(&p.purviews);
        let after = resolve_patterns(&ids, rows.as_ref());
        self.purview_delta(nonce, before, after, ids, rows).await
    }
}
