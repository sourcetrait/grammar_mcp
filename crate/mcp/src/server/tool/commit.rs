use crate::*;

/// Parameters for `commit()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct CommitParams {
    /// The equip to commit: its `<author>/<name>`, or a builtin's name.
    pub namepath: String,
}

#[mcp::tool_router(router = commit_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(
        description = "Commit an equip's lib and rig from its source to the MCP's repository for live use."
    )]
    pub(crate) async fn commit(
        &self,
        mcp::Parameters(p): mcp::Parameters<CommitParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("commit", &p).to_nonce();
        let equip = resolve_builtin(&p.namepath);
        let lock = match self.rig_locks.lookup(&equip).await {
            Some(l) => l,
            None => {
                return error_to_call_result(
                    GrammarMcpError::EquipNotRegistered {
                        equip: p.namepath.clone(),
                    },
                    nonce,
                );
            }
        };
        let _guard = lock.write().await;
        let engine = self.lint_engine.current();
        match commit_equip(&equip, &engine) {
            Ok(result) => success(nonce, &result),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
