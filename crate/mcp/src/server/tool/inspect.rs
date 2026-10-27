use crate::*;

/// Parameters for `interface help`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct InspectParams {
    /// A namepath at any arity, or a PATTERN ending in `/` or `:`, or `*` / `.`.
    pub namepath: String,
}

/// Success result of `interface help`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct InspectResult {
    /// An exact namepath's documentation, or a pattern's `signatures` block.
    pub doc: InspectDoc,
}

impl NuSh {
    /// `interface help`: documentation for a rig, a module, a call, or a pattern.
    pub(crate) async fn help(
        &self,
        nonce: datum::Nonce,
        p: InspectParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let pattern = match NamepathStr::parse(&p.namepath) {
            Ok(NamepathStr::Pattern(pattern)) => pattern,
            Ok(NamepathStr::Namepath(_)) => {
                let command = EngineCommand::Help {
                    namepath: p.namepath,
                };
                return match self.engine.control(command).await {
                    Ok(ControlAnswer::Help(doc)) => success(nonce, &InspectResult { doc }),
                    Ok(_) => error_to_call_result(unexpected("help"), nonce),
                    Err(error) => error_to_call_result(error, nonce),
                };
            }
            Err(e) => return error_to_call_result(e, nonce),
        };
        let patterns = match pattern {
            NamepathPattern::Current => {
                let rows = match load_purviews() {
                    Ok(rows) => rows,
                    Err(error) => return error_to_call_result(error, nonce),
                };
                expand_values(
                    &resolve_patterns(&self.current_purview.ids(), rows.as_ref()),
                    rows.as_ref(),
                )
            }
            _ => vec![p.namepath],
        };
        match self.signatures(patterns).await {
            Ok(signatures) => success(
                nonce,
                &InspectResult {
                    doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
                },
            ),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
