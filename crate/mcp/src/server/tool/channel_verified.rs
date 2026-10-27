use crate::*;

/// Parameters for `channel_verified()` (none).
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct ChannelVerifiedParams {}

#[mcp::tool_router(router = channel_verified_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(
        description = "Confirm the channel/Open packet was seen; ends the verify window."
    )]
    pub(crate) async fn channel_verified(
        &self,
        mcp::Parameters(p): mcp::Parameters<ChannelVerifiedParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("channel_verified", &p).to_nonce();
        match self.channel.mark_verified() {
            Ok(()) => success_void(nonce),
            Err(ChannelVerifyError::NotOpen) => {
                error_to_call_result(GrammarMcpError::ChannelNotOpen, nonce)
            }
            Err(ChannelVerifyError::NotClaimed) => {
                error_to_call_result(GrammarMcpError::ChannelNotClaimed, nonce)
            }
        }
    }
}
