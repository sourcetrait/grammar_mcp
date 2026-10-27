use crate::*;

impl NuSh {
    /// `interface channel ready`: the channel/Open packet seen; verify it.
    pub(crate) async fn channel_ready(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.channel.mark_verified() {
            Ok(()) => success_void(nonce),
            Err(ChannelVerifyError::NotOpen) => {
                error_to_call_result(GrammarMcpError::ChannelNotOpen, nonce)
            }
            Err(ChannelVerifyError::NotClaimed(mode)) => {
                error_to_call_result(GrammarMcpError::ChannelNotClaimed { mode }, nonce)
            }
        }
    }
}
