use crate::*;

/// Parameters for `rerun()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct RerunParams {
    /// The nonce of an earlier `execute` call, whose definition this evaluates again.
    pub nonce: String,
    /// Fresh data as NUON text, as `execute` takes it.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

#[mcp::tool_router(router = rerun_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = "Re-evaluate an earlier `execute` call's definition with fresh data.")]
    pub(crate) async fn rerun(
        &self,
        mcp::Parameters(p): mcp::Parameters<RerunParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("rerun", &p);
        if !cereal::is_base62(&p.nonce) {
            return error_to_call_result(
                GrammarMcpError::RerunInvalidNonce {
                    nonce: p.nonce.clone(),
                    reason: "nonce must be base62".to_string(),
                },
                nonce.to_nonce(),
            );
        }
        let Ok(definition) = fs::read_to_string(definition_file(&p.nonce)) else {
            return error_to_call_result(
                GrammarMcpError::RerunBodyMissing {
                    nonce: p.nonce.clone(),
                },
                nonce.to_nonce(),
            );
        };
        let engine = self.lint_engine.current();
        let prepared =
            match prepare_execute(&engine, &definition, &p.data, p.timeout.as_deref(), false) {
                Ok(prepared) => prepared,
                Err(error) => return error_to_call_result(error, nonce.to_nonce()),
            };
        let kind = InFlightKind::Rerun {
            source_nonce: p.nonce.clone(),
            lane: prepared.lane(),
        };
        let args_json = serde_json::json!({"data": p.data, "timeout": p.timeout});
        self.dispatch_execute(nonce, prepared, &definition, "rerun", kind, args_json)
            .await
    }
}
