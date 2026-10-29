use crate::*;

/// Parameters for `interface process kill`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct KillParams {
    /// The nonce of the in-flight call to cancel.
    pub nonce: String,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface process kill`: cancel an in-flight call by its nonce.
    pub(crate) async fn process_kill(
        &self,
        nonce: datum::Nonce,
        p: KillParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.system.control(EngineCommand::Kill { nonce: p.nonce }).await {
            Ok(_) => success_void(nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
