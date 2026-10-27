use crate::*;

/// Parameters for `execute()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct ExecuteParams {
    /// One Nu definition: `def execute [data: <type>]: nothing -> <type> { ... }`.
    pub definition: String,
    /// The definition's data as NUON text; `null` for `data: nothing`.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

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
        let answer = self
            .system
            .request(ExecuteRequest {
                host: Host::Local,
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
