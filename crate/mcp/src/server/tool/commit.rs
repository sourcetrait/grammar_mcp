use crate::*;

/// Parameters for `interface equip commit`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct CommitParams {
    /// The equip to commit: its `<author>/<name>`, or a builtin's name.
    pub namepath: String,
}

impl NuSh {
    /// `interface equip commit`: an equip's source promoted into the namespace.
    pub(crate) async fn equip_commit(
        &self,
        nonce: datum::Nonce,
        p: CommitParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
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
