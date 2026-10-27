use crate::*;

/// Parameters for `processes()` (none).
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct ProcessesParams {}

/// One in-flight tool usage in the `processes()` snapshot.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ProcessEntry {
    pub nonce: String,
    pub tool: String,
    pub started_at: u64,
    pub args: mcp::JsonObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Success result of `processes()`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ProcessesResult {
    pub processes: Vec<ProcessEntry>,
}

#[mcp::tool_router(router = processes_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = "List in-flight MCP tool usage.")]
    pub(crate) async fn processes(
        &self,
        mcp::Parameters(p): mcp::Parameters<ProcessesParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("processes", &p).to_nonce();
        let map = self.in_flight.lock().await;
        let entries: Vec<ProcessEntry> = map
            .iter()
            .map(|(nonce_str, entry)| {
                let args_obj = entry.args.as_object().cloned().unwrap_or_default();
                let (source_nonce, path) = match &entry.kind {
                    InFlightKind::Nu { .. } => (None, None),
                    InFlightKind::Rerun { source_nonce, .. } => (Some(source_nonce.clone()), None),
                    InFlightKind::Call { path } => (None, Some(path.clone())),
                };
                ProcessEntry {
                    nonce: nonce_str.clone(),
                    tool: entry.tool.to_string(),
                    started_at: entry.started_at,
                    args: args_obj,
                    source_nonce,
                    path,
                }
            })
            .collect();
        drop(map);
        success(nonce, &ProcessesResult { processes: entries })
    }
}
