use crate::*;

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface process kill`: cancel an in-flight call by its nonce.
    pub(crate) async fn process_kill(
        &self,
        nonce: datum::Nonce,
        p: KillParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.controller.control(EngineCommand::Kill { nonce: p.nonce }).await {
            Ok(_) => success_void(nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
