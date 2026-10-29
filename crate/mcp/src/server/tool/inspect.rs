use crate::*;

/// Parameters for `interface help`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct InspectParams {
    /// A namepath, a PATTERN ending in `/` or `:`, `*`, `.`, `interface`, or `grimm`.
    pub namepath: String,
}

/// Success result of `interface help`.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct InspectResult {
    /// An exact namepath's documentation, or a pattern's `signatures` block.
    pub doc: InspectDoc,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface help`: documentation for a rig, a module, a call, or a pattern.
    pub(crate) async fn help(
        &self,
        nonce: datum::Nonce,
        p: InspectParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let words = match p.namepath.as_str() {
            INTERFACE => Some(render_signatures(&config().deny)),
            GRIMM => Some(grimm_signatures()),
            _ => None,
        };
        if let Some(signatures) = words {
            return success(
                nonce,
                &InspectResult {
                    doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
                },
            );
        }
        let pattern = match NamepathStr::parse(&p.namepath) {
            Ok(NamepathStr::Pattern(pattern)) => pattern,
            Ok(NamepathStr::Namepath(_)) => {
                let command = EngineCommand::Help {
                    namepath: p.namepath,
                };
                return match self.system.control(command).await {
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
