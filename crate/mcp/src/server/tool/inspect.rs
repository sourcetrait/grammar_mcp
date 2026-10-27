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
        let namepath = match NamepathStr::parse(&p.namepath) {
            Ok(NamepathStr::Pattern(pattern)) => {
                let patterns = match pattern {
                    NamepathPattern::Current => {
                        let rows = match load_purviews() {
                            Ok(rows) => rows,
                            Err(error) => return error_to_call_result(error, nonce),
                        };
                        parse_patterns(&expand_values(
                            &resolve_patterns(&self.current_purview.ids(), rows.as_ref()),
                            rows.as_ref(),
                        ))
                    }
                    other => vec![NamepathStr::Pattern(other)],
                };
                let signatures =
                    render_signatures_within(&self.rig_locks, &patterns).await;
                return success(
                    nonce,
                    &InspectResult {
                        doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
                    },
                );
            }
            Ok(NamepathStr::Namepath(n)) => n,
            Err(e) => return error_to_call_result(e, nonce),
        };
        let (rig, module_path, name) = match namepath.validate() {
            Ok(NamepathRef::Rig { rig }) => (rig, String::new(), None),
            Ok(NamepathRef::Module {
                rig,
                module_path,
            }) => (rig, module_path, None),
            Ok(NamepathRef::Function {
                rig,
                module_path,
                name,
            }) => (rig, module_path, Some(name)),
            Err(e) => return error_to_call_result(e, nonce),
        };
        let lock = match self.rig_locks.lookup(&rig).await {
            Some(l) => l,
            None => {
                return error_to_call_result(
                    EngineError::EquipNotRegistered {
                        equip: rig.clone(),
                    },
                    nonce,
                );
            }
        };
        let _guard = lock.read().await;
        match inspect_impl(&rig, &module_path, name.as_deref()) {
            Ok(doc) => success(nonce, &InspectResult { doc }),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
