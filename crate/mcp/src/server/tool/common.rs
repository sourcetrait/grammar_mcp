use crate::*;

pub struct NuSh {
    /// mcp's system, which the engine's requests go through.
    pub(crate) system: Arc<McpHandle>,
    pub(crate) nonce_gen: Arc<datum::NonceGenerator>,
    /// This host process's identity, minted once at construction.
    pub(crate) mcp_nom: datum::NomPair,
    /// The engine an interface command parses on: its commands, nothing else.
    pub(crate) interface_engine: Arc<nu::EngineState>,
    /// The host's single packet channel; present but CLOSED at startup.
    pub(crate) channel: Arc<ChannelHandle>,
    /// Serializes `channel_open`'s decide-then-start across its await.
    pub(crate) channel_open_lock: Arc<tk::AsyncMutex<()>>,
    /// Which purview ids this host has in view; SESSION-resident.
    pub(crate) current_purview: Arc<CurrentPurview>,
    /// The process-wide registry of established remote links, keyed by alias.
    pub(crate) remote_links: Arc<std::sync::Mutex<HashMap<String, RemoteLinkEntry>>>,
    pub(crate) tool_router: mcp::ToolRouter<NuSh>,
}

/// One open remote link plus the peer address, for the registry and listing.
pub(crate) struct RemoteLinkEntry {
    pub handle: RemoteLinkHandle,
    pub addr: std::net::SocketAddr,
}

impl NuSh {
    pub(crate) fn new(
        nonce_gen: Arc<datum::NonceGenerator>,
        mcp_nom: datum::NomPair,
        system: Arc<McpHandle>,
        deny: &DenySet,
    ) -> Self {
        Self {
            system,
            nonce_gen,
            mcp_nom,
            interface_engine: Arc::new(interface_engine(deny)),
            channel: channel_handle(),
            channel_open_lock: Arc::new(tk::AsyncMutex::new(())),
            current_purview: Arc::new(CurrentPurview::new()),
            remote_links: remote_links(),
            tool_router: Self::tool_router(),
        }
    }
}

impl NuSh {
    /// A call's nonce, minted first from its tool and its parameters.
    pub(crate) fn mint<P: ser::Serialize>(
        &self,
        tool: &str,
        params: &P,
    ) -> datum::NoncePair {
        let payload = json::to_vec(params).unwrap_or_default();
        self.nonce_gen.generate_with(&(tool, payload)).into_pair()
    }

    /// The signature block within pattern strings, as the engine renders it.
    pub(crate) async fn signatures(&self, patterns: Vec<String>) -> GrammarMcpResult<String> {
        match self.system.control(EngineCommand::Signatures { patterns }).await? {
            ControlAnswer::Signatures(block) => Ok(block),
            _ => Err(unexpected("signatures")),
        }
    }
}

/// The internal error an answer of another command's kind is.
pub(crate) fn unexpected(command: &str) -> GrammarMcpError {
    GrammarMcpError::Internal {
        phase: format!("engine::{command}"),
        reason: "the engine answered another command".to_string(),
    }
}

/// An envelope, `nonce` first and one more field, as the result's one NUON text.
pub(crate) fn envelope_text(
    nonce: datum::Nonce,
    field: &str,
    value: nu::Value,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    let span = nu::Span::unknown();
    let mut envelope = nu::Record::new();
    envelope.insert("nonce", nu::Value::string(nonce.to_string(), span));
    envelope.insert(field, value);
    let text = render_nuon(&nu::Value::record(envelope, span))
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope render: {e}"), None))?;
    Ok(mcp::CallToolResult::success(vec![mcp::ContentBlock::text(text)]))
}

/// A tool's record as a nu value, through the serde bridge.
pub(crate) fn serde_to_nu<T: ser::Serialize>(value: &T) -> Result<nu::Value, mcp::ErrorData> {
    let json = json::to_value(value)
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope serialize: {e}"), None))?;
    Ok(json_value_to_nu_value(&json))
}

/// A success, `{nonce, result}`: the result a nu value the tool built.
pub(crate) fn success_value(
    nonce: datum::Nonce,
    result: nu::Value,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    envelope_text(nonce, "result", result)
}

/// A success, `{nonce, result}`: the result a tool's record.
pub(crate) fn success<T: ser::Serialize>(
    nonce: datum::Nonce,
    result: &T,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    success_value(nonce, serde_to_nu(result)?)
}

/// A success with nothing to answer, `{nonce, result: null}`.
pub(crate) fn success_void(nonce: datum::Nonce) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    success_value(nonce, nu::Value::nothing(nu::Span::unknown()))
}

/// A call's value from the engine, its nushell form, or why it has none.
pub(crate) fn answer_value(answer: EngineResult<ValueAnswer>) -> GrammarMcpResult<nu::Value> {
    let value = answer?.result?;
    value
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| GrammarMcpError::Internal {
            phase: "engine::value".to_string(),
            reason: e.to_string(),
        })
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
