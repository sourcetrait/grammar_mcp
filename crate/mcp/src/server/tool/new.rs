use crate::*;

/// Parameters for `new()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct NewParams {
    /// Namepaths to scaffold into installed equips' rigs: a module or a function.
    pub namepaths: Vec<String>,
}

/// Success result of `new()` -- every path created across the batch.
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewResult {
    pub created: Vec<String>,
}

#[mcp::tool_router(router = new_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(
        name = "new",
        description = "Scaffold modules / functions (by namepath) into an equip's rig."
    )]
    pub(crate) async fn scaffold(
        &self,
        mcp::Parameters(p): mcp::Parameters<NewParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("new", &p).to_nonce();
        let mut targets: Vec<(String, String, Option<String>)> = Vec::new();
        for np in &p.namepaths {
            match Namepath(np.clone()).validate() {
                Ok(NamepathRef::Module {
                    rig,
                    module_path,
                }) => targets.push((rig, module_path, None)),
                Ok(NamepathRef::Function {
                    rig,
                    module_path,
                    name,
                }) => targets.push((rig, module_path, Some(name))),
                Ok(NamepathRef::Rig { .. }) => {
                    return error_to_call_result(
                        GrammarMcpError::NamepathInvalid {
                            namepath: np.clone(),
                            reason: "new() needs a module or function namepath; use equip(new) to create an equip"
                                .to_string(),
                        },
                        nonce,
                    );
                }
                Err(e) => return error_to_call_result(e, nonce),
            }
        }

        let mut libs: Vec<String> = targets.iter().map(|(l, _, _)| l.clone()).collect();
        libs.sort();
        libs.dedup();
        let mut locks = Vec::new();
        for lib in &libs {
            match self.rig_locks.lookup(lib).await {
                Some(l) => locks.push(l),
                None => {
                    return error_to_call_result(
                        GrammarMcpError::EquipNotRegistered {
                            equip: lib.clone(),
                        },
                        nonce,
                    );
                }
            }
        }
        let mut guards = Vec::new();
        for l in &locks {
            guards.push(l.write().await);
        }

        for (rig, module_path, name) in &targets {
            match scaffold_leaf_exists(rig, module_path, name.as_deref()) {
                Ok(true) => {
                    return error_to_call_result(
                        GrammarMcpError::RigInvalidName {
                            name: module_path.clone(),
                            reason: format!(
                                "`{}` already exists; edit it instead of scaffolding over it",
                                name.clone().unwrap_or_else(|| module_path.clone()),
                            ),
                        },
                        nonce,
                    );
                }
                Ok(false) => {}
                Err(e) => return error_to_call_result(e, nonce),
            }
        }

        let mut created = Vec::new();
        for (rig, module_path, name) in &targets {
            match scaffold_leaf(rig, module_path, name.as_deref()) {
                Ok(mut c) => created.append(&mut c),
                Err(e) => return error_to_call_result(e, nonce),
            }
        }
        success(nonce, &NewResult { created })
    }
}
