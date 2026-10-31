use crate::*;

/// Parameters for `execute()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct ExecuteParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// One Nu definition: `def execute [data: <type>]: nothing -> <type> { ... }`.
    pub definition: String,
    /// The definition's data as NUON text; `null` for `data: nothing`.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

#[cfg(feature = "embedded-cli")]
#[mcp::tool_router(router = execute_router, vis = "pub(crate)")]
impl NuSh {
    #[allow(clippy::doc_overindented_list_items)]
    #[doc = include_str!("../../../../../include/human/mcp/execute.md")]
    #[mcp::tool(name = "execute")]
    pub(crate) async fn execute(
        &self,
        mcp::Parameters(p): mcp::Parameters<ExecuteParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("execute", &p).to_nonce();
        let host = match resolve_host(&p.host) {
            Ok(host) => host,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let answer = self
            .controller
            .request(ExecuteRequest {
                host,
                nonce,
                definition: p.definition,
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
