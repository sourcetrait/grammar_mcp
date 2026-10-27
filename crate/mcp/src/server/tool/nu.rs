use crate::*;

/// Parameters for `nu()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct NuParams {
    /// One Nu definition: `def execute [data: <type>]: nothing -> <type> { ... }`.
    pub definition: String,
    /// The definition's data as NUON text; `null` for `data: nothing`.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

/// A `nu` call ready to dispatch: its contract, its source, and its timeout.
pub(crate) struct PreparedNu {
    def: ExecuteDef,
    source: String,
    timeout_ms: u64,
}

impl PreparedNu {
    /// The lane the call runs on.
    pub(crate) fn lane(&self) -> Lane {
        self.def.lane
    }
}

/// The call's timeout as a positive Nu duration: its milliseconds and its value.
fn parse_timeout(text: Option<&str>) -> Result<(u64, nu::Value), GrammarMcpError> {
    let span = nu::Span::unknown();
    let Some(text) = text else {
        let nanos = DEFAULT_TIMEOUT_MS as i64 * 1_000_000;
        return Ok((DEFAULT_TIMEOUT_MS, nu::Value::duration(nanos, span)));
    };
    let invalid = |reason: String| GrammarMcpError::TimeoutInvalid { reason };
    let value = nu::from_nuon(text, None)
        .map_err(|e| invalid(format!("`{text}` is not a Nu duration: {e}")))?;
    match value {
        nu::Value::Duration { val, .. } if val > 0 => {
            Ok(((val as u64).div_ceil(1_000_000), nu::Value::duration(val, span)))
        }
        other => Err(invalid(format!(
            "`{text}` is a {}, not a positive duration such as 30sec",
            other.get_type(),
        ))),
    }
}

/// Hold a call's definition, data, and timeout to the contract and build its source.
pub(crate) fn prepare_nu(
    engine: &ParseEngine,
    definition: &str,
    data: &str,
    timeout: Option<&str>,
    lint: bool,
) -> Result<PreparedNu, GrammarMcpError> {
    let def = analyze_definition(engine, definition, lint)
        .map_err(|diagnostics| GrammarMcpError::DefinitionViolations { diagnostics })?;
    let invalid = |reason: String| GrammarMcpError::DataInvalid { reason };
    if data.trim().is_empty() {
        return Err(invalid(
            "data is NUON text, `null` for a `data: nothing` definition".to_string(),
        ));
    }
    let value = nu::from_nuon(data, None).map_err(|e| invalid(format!("data is not NUON: {e}")))?;
    check_value(&value, &def.data_type, "data").map_err(invalid)?;
    let data_literal = render_nuon(&value).map_err(invalid)?;
    let (timeout_ms, timeout_value) = parse_timeout(timeout)?;
    let timeout_literal = if def.takes_timeout {
        let literal = render_nuon(&timeout_value)
            .map_err(|reason| GrammarMcpError::TimeoutInvalid { reason })?;
        Some(literal)
    } else {
        None
    };
    let source = build_nu_source(definition, &data_literal, timeout_literal.as_deref());
    Ok(PreparedNu {
        def,
        source,
        timeout_ms,
    })
}

impl NuSh {
    /// Dispatch a prepared call on its lane and hold its result to the contract.
    pub(crate) async fn dispatch_nu(
        &self,
        nonce: datum::NoncePair,
        prepared: PreparedNu,
        definition: &str,
        tool: &'static str,
        kind: InFlightKind,
        args_json: json::Value,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let PreparedNu {
            def,
            source,
            timeout_ms,
        } = prepared;
        let dispatched = match def.lane {
            Lane::Stateless => {
                dispatch_pooled(
                    &self.executor,
                    &self.in_flight,
                    &self.hung_watch,
                    CacheKind::Nu,
                    nonce,
                    source,
                    tool,
                    args_json,
                    kind,
                    Some(definition),
                    Some(timeout_ms),
                )
                .await
            }
            Lane::Stateful => {
                dispatch_stateful(
                    &self.stateful_engine,
                    &self.env_jobs,
                    &self.in_flight,
                    &self.hung_watch,
                    nonce,
                    source,
                    tool,
                    args_json,
                    kind,
                    Some(definition),
                    Some(timeout_ms),
                )
                .await
            }
        };
        let outcome = match dispatched {
            Ok(outcome) => outcome,
            Err(de) => return error_to_call_result(de.error, de.nonce),
        };
        if let Err(reason) = check_value(&outcome.value, &def.result_type, "result") {
            return error_to_call_result(GrammarMcpError::ResultInvalid { reason }, outcome.nonce);
        }
        success_value(outcome.nonce, outcome.value)
    }
}

#[mcp::tool_router(router = nu_router, vis = "pub(crate)")]
impl NuSh {
    #[allow(clippy::doc_overindented_list_items)]
    #[doc = include_str!("../../../../../include/human/mcp/execute.md")]
    #[mcp::tool(name = "nu")]
    pub(crate) async fn nu(
        &self,
        mcp::Parameters(p): mcp::Parameters<NuParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("nu", &p);
        let engine = self.lint_engine.current();
        let prepared =
            match prepare_nu(&engine, &p.definition, &p.data, p.timeout.as_deref(), true) {
                Ok(prepared) => prepared,
                Err(error) => return error_to_call_result(error, nonce.to_nonce()),
            };
        let kind = InFlightKind::Nu {
            lane: prepared.lane(),
        };
        let args_json = serde_json::json!({"data": p.data, "timeout": p.timeout});
        self.dispatch_nu(nonce, prepared, &p.definition, "nu", kind, args_json)
            .await
    }
}
