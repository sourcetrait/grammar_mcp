use crate::*;

/// Parameters for `interface equip commit`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct CommitParams {
    /// The equip to commit: its `<author>/<name>`, or a builtin's name.
    pub namepath: String,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface equip commit`: an equip's source promoted into the namespace.
    pub(crate) async fn equip_commit(
        &self,
        nonce: datum::Nonce,
        p: CommitParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let command = EngineCommand::EquipCommit {
            namepath: p.namepath,
        };
        match self.system.control(command).await {
            Ok(ControlAnswer::Committed(result)) => success(nonce, &result),
            Ok(_) => error_to_call_result(unexpected("equip_commit"), nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
