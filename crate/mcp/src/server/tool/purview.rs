use crate::*;

/// Success result of `interface purview set`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewSetResult {
    /// `info`'s `signatures` for what this call brought INTO view, or null.
    pub revealed: Option<String>,
}

/// Parameters for `interface purview set`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct PurviewParams {
    /// The purview ids to put in view, REPLACING the current set.
    pub purviews: Vec<String>,
}

impl NuSh {
    /// `interface purview set`: replace the current view with these purviews.
    pub(crate) async fn purview_set(
        &self,
        nonce: datum::Nonce,
        p: PurviewParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let rows = match load_purviews() {
            Ok(rows) => rows,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let want: Vec<String> = p
            .purviews
            .iter()
            .map(|id| purview_ref(id).unwrap_or(id).to_string())
            .collect();
        for id in &want {
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
        let ids = self.current_purview.set(&want);
        let after = resolve_patterns(&ids, rows.as_ref());
        let (revealed, _hidden) = pattern_delta(
            &expand_values(&before, rows.as_ref()),
            &expand_values(&after, rows.as_ref()),
        );
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
