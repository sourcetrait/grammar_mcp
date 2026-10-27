use crate::*;

/// Parameters for `call()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct CallParams {
    /// The function's namepath: `rig:module/path:function`.
    pub namepath: String,
    /// JSON object of argument values passed to the function as `$args`.
    pub args: mcp::JsonObject,
    /// Optional per-call timeout in milliseconds; defaults to 120000 (2 minutes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[mcp::tool_router(router = call_router, vis = "pub(crate)")]
impl NuSh {
    #[doc = include_str!("../../../../../include/mcp/tool/call.md")]
    #[mcp::tool(description = "Invoke a committed rig function with typed args.")]
    pub(crate) async fn call(
        &self,
        mcp::Parameters(p): mcp::Parameters<CallParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("call", &p).to_nonce();
        let args = json_value_to_nu_value(&json::Value::Object(p.args));
        let args = match NuVal::try_from_nu(args) {
            Ok(args) => args,
            Err(e) => {
                return error_to_call_result(
                    GrammarMcpError::Internal {
                        phase: "call::args".to_string(),
                        reason: e.to_string(),
                    },
                    nonce,
                );
            }
        };
        let answer = self
            .system
            .request(CallRequest {
                host: Host::Local,
                nonce,
                namepath: p.namepath,
                args,
                timeout_ms: p.timeout_ms,
            })
            .await;
        match answer_value(answer) {
            Ok(value) => success_value(nonce, value).or_else(|_| success_value(nonce, empty_record())),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}

fn empty_record() -> nu::Value {
    nu::Value::record(nu::Record::new(), nu::Span::unknown())
}
