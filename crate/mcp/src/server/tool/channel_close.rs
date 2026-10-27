use crate::*;

/// RFC 6455 normal closure.
const CLOSE_PLANNED: u16 = 1000;
const PLANNED_REASON: &str = "channel closed";

impl NuSh {
    /// `interface channel close`: close the packet channel and its inbox.
    pub(crate) async fn channel_close(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.channel.close(CLOSE_PLANNED, PLANNED_REASON);
        if let Some(inbox) = self.channel.take_inbox()
            && let Err(e) = fs::remove_dir_all(&inbox)
        {
            eprintln!("grammar: inbox prune failed at {}: {e}", inbox.display());
        }
        success_void(nonce)
    }
}
