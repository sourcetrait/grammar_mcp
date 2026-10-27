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

/// Success result of `call()`.
#[derive(Debug, ser::Serialize, schema::JsonSchema)]
pub(crate) struct CallEnvelope {
    pub result: mcp::JsonObject,
    pub nonce: String,
}

#[mcp::tool_router(router = call_router, vis = "pub(crate)")]
impl NuSh {
    #[doc = include_str!("../../../../../include/mcp/tool/call.md")]
    #[mcp::tool(
        description = "Invoke a committed rig function with typed args.",
        output_schema = tool_output_schema::<CallEnvelope>()
    )]
    pub(crate) async fn call(
        &self,
        mcp::Parameters(p): mcp::Parameters<CallParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let (rig, module_path, name) = match Namepath(p.namepath.clone()).validate() {
            Ok(NamepathRef::Function {
                rig,
                module_path,
                name,
            }) => (rig, module_path, name),
            Ok(_) => {
                return Ok(error_to_call_result(
                    GrammarMcpError::NamepathInvalid {
                        namepath: p.namepath.clone(),
                        reason: "call requires a function namepath: rig:module/path:function"
                            .to_string(),
                    },
                    None,
                ));
            }
            Err(e) => return Ok(error_to_call_result(e, None)),
        };
        let lock = match self.rig_locks.lookup(&rig).await {
            Some(l) => l,
            None => {
                return Ok(error_to_call_result(
                    GrammarMcpError::EquipNotRegistered {
                        equip: rig.clone(),
                    },
                    None,
                ));
            }
        };
        let _guard = lock.read().await;
        if let Err(e) = require_rig(&rig) {
            return Ok(error_to_call_result(e, None));
        }
        let index = match load_index(&rig) {
            Ok(i) => i,
            Err(e) => {
                return Ok(error_to_call_result(
                    GrammarMcpError::Internal {
                        phase: "call::load_index".to_string(),
                        reason: e.to_string(),
                    },
                    None,
                ));
            }
        };
        let is_call_target = index_node(&index, &module_path)
            .map(|(fns, _)| fns.iter().any(|f| f.name == name))
            .unwrap_or(false);
        if !is_call_target {
            return Ok(error_to_call_result(
                GrammarMcpError::FunctionNotDefined {
                    rig: rig.clone(),
                    module_path: module_path.clone(),
                    name: name.clone(),
                },
                None,
            ));
        }
        let payload_bytes = match json::to_vec(&p) {
            Ok(b) => b,
            Err(e) => {
                return Ok(error_to_call_result(
                    GrammarMcpError::Internal {
                        phase: "call::serialize_payload".to_string(),
                        reason: e.to_string(),
                    },
                    None,
                ));
            }
        };
        let nonce = self.nonce_gen.generate_with(&payload_bytes).into_pair();
        let source = build_call_source(&rig, &module_path, &name, &p.args);
        let path_str = p.namepath.clone();
        let args_json = serde_json::Value::Object(p.args.clone());
        let outcome = match dispatch_pooled(
            &self.executor,
            &self.in_flight,
            &self.hung_watch,
            CacheKind::Calls,
            nonce,
            source,
            "call",
            args_json,
            InFlightKind::Call { path: path_str },
            None,
            p.timeout_ms,
        )
        .await
        {
            Ok(o) => o,
            Err(de) => return Ok(error_to_call_result(de.error, de.nonce)),
        };
        envelope_to_structured(&CallEnvelope {
            result: result_object(outcome.value),
            nonce: outcome.nonce.into_pair().as_str().to_string(),
        })
    }
}

/// A call's value as the JSON object its envelope carries, anything else `{}`.
fn result_object(value: nu::Value) -> mcp::JsonObject {
    nu::JsonValue::from_value(value)
        .ok()
        .and_then(|value| json::to_value(&value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}
