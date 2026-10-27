use crate::*;

/// Parameters for `interface purview configure`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct PurviewConfigureParams {
    /// The purview to set, created if absent; a path-like snake label.
    pub purview: String,
    /// Its namepath patterns; an EMPTY list DELETES the purview.
    pub namepaths: Vec<String>,
}

/// Success result of `interface purview configure`, the state after the write.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewConfigureResult {
    /// Everything this purview reveals; null when the call deleted it.
    pub signatures: Option<String>,
}

impl NuSh {
    /// `interface purview configure`: set a purview's patterns; empty deletes it.
    pub(crate) async fn purview_configure(
        &self,
        nonce: datum::Nonce,
        p: PurviewConfigureParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        if is_derived_purview(&p.purview) {
            return error_to_call_result(
                GrammarMcpError::PurviewInvalidId {
                    id: p.purview.clone(),
                    reason: "`.` and `*` are derived, not stored, so they cannot be configured"
                        .to_string(),
                },
                nonce,
            );
        }
        if !is_valid_purview_id(&p.purview) {
            return error_to_call_result(
                GrammarMcpError::PurviewInvalidId {
                    id: p.purview.clone(),
                    reason: "must be slash-separated snake components, bare relative \
                             (no leading `/` or `./`)"
                        .to_string(),
                },
                nonce,
            );
        }
        for value in &p.namepaths {
            if let Some(id) = purview_ref(value)
                && !is_valid_purview_ref(id)
            {
                return error_to_call_result(
                    GrammarMcpError::PurviewInvalidId {
                        id: value.clone(),
                        reason: "`@` must reference a configurable purview; `@.` and `@*` \
                                 are derived and are never rows"
                            .to_string(),
                    },
                    nonce,
                );
            }
        }
        let rows = {
            let _repo = equip_repo_lock();
            let mut rows = match load_purviews() {
                Ok(rows) => rows.unwrap_or_default(),
                Err(error) => return error_to_call_result(error, nonce),
            };
            rows.retain(|row| row.id != p.purview);
            let namepaths = if p.namepaths.is_empty() && p.purview == PURVIEW_DEFAULT {
                vec![PURVIEW_ALL.to_string()]
            } else {
                p.namepaths.clone()
            };
            if !namepaths.is_empty() {
                rows.push(PurviewRow {
                    id: p.purview.clone(),
                    namepath_patterns: namepaths,
                });
            }
            prune_dangling(&mut rows);
            if let Err(error) = save_purviews(&rows) {
                return error_to_call_result(error, nonce);
            }
            rows
        };
        self.current_purview.retain_known(Some(&rows));
        let values = rows
            .iter()
            .find(|row| row.id == p.purview)
            .map(|row| row.namepath_patterns.clone());
        let signatures = match values {
            None => None,
            Some(values) => match self.signatures(expand_values(&values, Some(&rows))).await {
                Ok(signatures) => Some(signatures),
                Err(error) => return error_to_call_result(error, nonce),
            },
        };
        success(nonce, &PurviewConfigureResult { signatures })
    }
}
