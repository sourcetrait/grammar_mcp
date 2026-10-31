use crate::*;

/// Parameters for `call()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct CallParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// The function's namepath: `rig:module/path:function`.
    pub namepath: String,
    /// The function's data as NUON text; `null` for `data: nothing`.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

/// The tool's description, the same on the client and the test driver.
pub(crate) const CALL_DESCRIPTION: &str = "Invoke a committed rig function on its typed data.";

#[cfg(feature = "embedded-cli")]
#[mcp::tool_router(router = call_router, vis = "pub(crate)")]
impl NuSh {
    #[doc = include_str!("../../../../../include/mcp/tool/call.md")]
    #[mcp::tool(description = CALL_DESCRIPTION)]
    pub(crate) async fn call(
        &self,
        mcp::Parameters(p): mcp::Parameters<CallParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("call", &p).to_nonce();
        let host = match resolve_host(&p.host) {
            Ok(host) => host,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let answer = self
            .controller
            .request(CallRequest {
                host,
                nonce,
                namepath: p.namepath,
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
