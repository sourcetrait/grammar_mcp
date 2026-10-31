use crate::*;

/// Parameters for `rerun()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct RerunParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// The nonce of an earlier `execute` call, whose definition this evaluates again.
    pub nonce: String,
    /// Fresh data as NUON text, as `execute` takes it.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

/// The tool's description, the same on the client and the test driver.
pub(crate) const RERUN_DESCRIPTION: &str =
    "Re-evaluate an earlier `execute` call's definition with fresh data.";

#[cfg(feature = "embedded-cli")]
#[mcp::tool_router(router = rerun_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = RERUN_DESCRIPTION)]
    pub(crate) async fn rerun(
        &self,
        mcp::Parameters(p): mcp::Parameters<RerunParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("rerun", &p).to_nonce();
        let host = match resolve_host(&p.host) {
            Ok(host) => host,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let answer = self
            .controller
            .request(RerunRequest {
                host,
                nonce,
                source: p.nonce,
                data: p.data,
                timeout: p.timeout,
            })
            .await;
        match answer_value(answer) {
            Ok(value) => success_value(nonce, value),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
