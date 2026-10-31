use crate::*;

/// The test driver's server: the four tools on the in-process engine.
#[cfg(feature = "embedded-cli")]
pub struct NuSh {
    /// mcp's system, which the engine's requests go through.
    pub(crate) system: Arc<McpHandle>,
    pub(crate) nonce_gen: Arc<datum::NonceGenerator>,
    /// This host process's identity, minted once at construction.
    pub(crate) mcp_nom: datum::NomPair,
    /// The engine an interface command parses on: its commands, nothing else.
    pub(crate) interface_engine: Arc<nu::EngineState>,
    /// The Channel's and the remote channels' commands, the client's too.
    pub(crate) face: ChannelFace,
    /// Which purview ids this host has in view; SESSION-resident.
    pub(crate) current_purview: Arc<CurrentPurview>,
    pub(crate) tool_router: mcp::ToolRouter<NuSh>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    pub(crate) fn new(
        face: ChannelFace,
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
            face,
            current_purview: Arc::new(CurrentPurview::new()),
            tool_router: Self::tool_router(),
        }
    }

    /// A call's nonce, minted first from its tool and its parameters.
    pub(crate) fn mint<P: ser::Serialize>(
        &self,
        tool: &str,
        params: &P,
    ) -> datum::NoncePair {
        mint_nonce(&self.nonce_gen, tool, params)
    }

    /// The signature block within pattern strings, as the engine renders it.
    pub(crate) async fn signatures(&self, patterns: Vec<String>) -> GrammarMcpResult<String> {
        match self.system.control(EngineCommand::Signatures { patterns }).await? {
            ControlAnswer::Signatures(block) => Ok(block),
            _ => Err(unexpected("signatures")),
        }
    }
}

/// A tool's `host` alias as the engine's host; `local` is the one known.
pub(crate) fn resolve_host(alias: &str) -> GrammarMcpResult<Host> {
    if alias == LOCAL_HOST {
        Ok(Host::Local)
    } else {
        Err(GrammarMcpError::HostUnknown {
            host: alias.to_string(),
        })
    }
}

/// The internal error an answer of another command's kind is.
#[cfg(feature = "embedded-cli")]
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

/// A tool's record as a nu value, written by serde as a NuVal through nuval.
pub(crate) fn serde_to_nu<T: ser::Serialize>(value: &T) -> Result<nu::Value, mcp::ErrorData> {
    nuval::to_nuval(value)
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope serialize: {e}"), None))?
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope convert: {e}"), None))
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
#[cfg(feature = "embedded-cli")]
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
