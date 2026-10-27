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
        let pair = self.mint("call", &p);
        let nonce = pair.to_nonce();
        let (rig, module_path, name) = match Namepath(p.namepath.clone()).validate() {
            Ok(NamepathRef::Function {
                rig,
                module_path,
                name,
            }) => (rig, module_path, name),
            Ok(_) => {
                return error_to_call_result(
                    GrammarMcpError::NamepathInvalid {
                        namepath: p.namepath.clone(),
                        reason: "call requires a function namepath: rig:module/path:function"
                            .to_string(),
                    },
                    nonce,
                );
            }
            Err(e) => return error_to_call_result(e, nonce),
        };
        let lock = match self.rig_locks.lookup(&rig).await {
            Some(l) => l,
            None => {
                return error_to_call_result(
                    GrammarMcpError::EquipNotRegistered {
                        equip: rig.clone(),
                    },
                    nonce,
                );
            }
        };
        let _guard = lock.read().await;
        if let Err(e) = require_rig(&rig) {
            return error_to_call_result(e, nonce);
        }
        let index = match load_index(&rig) {
            Ok(i) => i,
            Err(e) => {
                return error_to_call_result(
                    GrammarMcpError::Internal {
                        phase: "call::load_index".to_string(),
                        reason: e.to_string(),
                    },
                    nonce,
                );
            }
        };
        let is_call_target = index_node(&index, &module_path)
            .map(|(fns, _)| fns.iter().any(|f| f.name == name))
            .unwrap_or(false);
        if !is_call_target {
            return error_to_call_result(
                GrammarMcpError::FunctionNotDefined {
                    rig: rig.clone(),
                    module_path: module_path.clone(),
                    name: name.clone(),
                },
                nonce,
            );
        }
        let source = build_call_source(&rig, &module_path, &name, &p.args);
        let path_str = p.namepath.clone();
        let args_json = serde_json::Value::Object(p.args.clone());
        let outcome = match dispatch_pooled(
            &self.executor,
            &self.in_flight,
            &self.hung_watch,
            CacheKind::Calls,
            pair,
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
            Err(de) => return error_to_call_result(de.error, de.nonce),
        };
        success_value(nonce, lenient_result(outcome.value))
            .or_else(|_| success_value(nonce, empty_record()))
    }
}

/// A call's value as its envelope's result: nothing, a record, or a table as it
/// is, anything else `{}`.
fn lenient_result(value: nu::Value) -> nu::Value {
    let kept = match &value {
        nu::Value::Nothing { .. } | nu::Value::Record { .. } => true,
        nu::Value::List { vals, .. } => vals.iter().all(|v| matches!(v, nu::Value::Record { .. })),
        _ => false,
    };
    if kept { value } else { empty_record() }
}

fn empty_record() -> nu::Value {
    nu::Value::record(nu::Record::new(), nu::Span::unknown())
}
