use crate::*;

/// Parameters for `interface channel config`, a PARTIAL update.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct ConfigChannelParams {
    /// Window for the SOFT threshold, in whole seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_warn_window_secs: Option<u64>,
    /// Sends within that window before one warning is issued for an origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_warn_rate: Option<u32>,
    /// Window for the HARD threshold, in whole seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_error_window_secs: Option<u64>,
    /// Sends within that window before the origin is refused and stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spam_error_rate: Option<u32>,
}

/// The policy now IN FORCE, whether or not this call changed it.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ConfigChannelResult {
    pub spam_warn_window_secs: u64,
    pub spam_warn_rate: u32,
    pub spam_error_window_secs: u64,
    pub spam_error_rate: u32,
}

impl From<SpamThresholds> for ConfigChannelResult {
    fn from(spam: SpamThresholds) -> Self {
        Self {
            spam_warn_window_secs: spam.warn_window.as_secs(),
            spam_warn_rate: spam.warn_rate,
            spam_error_window_secs: spam.error_window.as_secs(),
            spam_error_rate: spam.error_rate,
        }
    }
}

impl NuSh {
    /// `interface channel config`: read or adjust the send-rate thresholds.
    pub(crate) async fn channel_config(
        &self,
        nonce: datum::Nonce,
        p: ConfigChannelParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.channel.set_thresholds(
            p.spam_warn_window_secs,
            p.spam_warn_rate,
            p.spam_error_window_secs,
            p.spam_error_rate,
        ) {
            Ok(spam) => success(nonce, &ConfigChannelResult::from(spam)),
            Err(reason) => error_to_call_result(GrammarMcpError::SchemaInvalid { reason }, nonce),
        }
    }
}
