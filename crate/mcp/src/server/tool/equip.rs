use crate::*;

/// Parameters for `equip()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct EquipParams {
    /// One of: `new`, `install`, `check`, `uninstall`.
    pub action: String,
    /// The equip's source: an `equip/<author>/<name>` directory.
    pub source_dir: String,
}

/// `new` summary: the paths scaffolded for the fresh equip's rig.
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewSummary {
    pub created: Vec<String>,
}

/// `check` summary: `ok` is true iff there are no error rows.
#[derive(Debug, ser::Serialize)]
pub(crate) struct CheckSummary {
    pub ok: bool,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

/// The per-action summary, `install`'s its commit's; `uninstall` returns none.
#[derive(Debug, ser::Serialize)]
#[serde(untagged)]
pub(crate) enum EquipSummary {
    New(NewSummary),
    Install(CommitResult),
    Check(CheckSummary),
}

/// Success result of `equip()`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct EquipResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<EquipSummary>,
}

/// The actions `equip()` takes.
const ACTIONS: [&str; 4] = ["new", "install", "check", "uninstall"];

#[mcp::tool_router(router = equip_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(
        description = "Equip administration on an equip/<author>/<name> source dir: new, install, check, uninstall."
    )]
    pub(crate) async fn equip(
        &self,
        mcp::Parameters(p): mcp::Parameters<EquipParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("equip", &p).to_nonce();
        if !ACTIONS.contains(&p.action.as_str()) {
            return error_to_call_result(
                GrammarMcpError::EquipInvalidAction {
                    action: p.action.clone(),
                },
                nonce,
            );
        }
        let source_dir = Path::new(&p.source_dir);
        let equip = match equip_namepath_of(source_dir) {
            Ok(equip) => equip,
            Err(e) => return error_to_call_result(e, nonce),
        };
        match p.action.as_str() {
            "new" => {
                let lock = match self.rig_locks.register(&equip).await {
                    Ok(l) => l,
                    Err(_) => {
                        return error_to_call_result(
                            GrammarMcpError::EquipAlreadyRegistered { equip },
                            nonce,
                        );
                    }
                };
                let _guard = lock.write().await;
                match establish_equip(&equip, source_dir) {
                    Ok(created) => success(
                        nonce,
                        &EquipResult {
                            summary: Some(EquipSummary::New(NewSummary { created })),
                        },
                    ),
                    Err(e) => {
                        self.rig_locks.unregister(&equip).await;
                        error_to_call_result(e, nonce)
                    }
                }
            }
            "install" => {
                let lock = match self.rig_locks.register(&equip).await {
                    Ok(l) => l,
                    Err(_) => {
                        return error_to_call_result(
                            GrammarMcpError::EquipAlreadyRegistered { equip },
                            nonce,
                        );
                    }
                };
                let _guard = lock.write().await;
                let engine = self.lint_engine.current();
                match install_equip(&equip, source_dir, &engine) {
                    Ok(result) => {
                        if rig_dir(&equip).exists() {
                            purview_add_rig(&equip, &self.current_purview.ids());
                        }
                        success(
                            nonce,
                            &EquipResult {
                                summary: Some(EquipSummary::Install(result)),
                            },
                        )
                    }
                    Err(e) => {
                        self.rig_locks.unregister(&equip).await;
                        error_to_call_result(e, nonce)
                    }
                }
            }
            "check" => {
                let Some(lock) = self.rig_locks.lookup(&equip).await else {
                    return error_to_call_result(
                        GrammarMcpError::EquipNotRegistered { equip },
                        nonce,
                    );
                };
                let _guard = lock.read().await;
                if let Err(e) = check_source_dir(&equip, source_dir) {
                    return error_to_call_result(e, nonce);
                }
                let engine = self.lint_engine.current();
                match check_equip(&equip, &engine) {
                    Ok(result) => success(
                        nonce,
                        &EquipResult {
                            summary: Some(EquipSummary::Check(check_summary_from(&result))),
                        },
                    ),
                    Err(e) => error_to_call_result(e, nonce),
                }
            }
            _ => {
                let Some(lock) = self.rig_locks.lookup(&equip).await else {
                    return success(nonce, &EquipResult { summary: None });
                };
                let guard = lock.write().await;
                if let Err(e) = check_source_dir(&equip, source_dir) {
                    return error_to_call_result(e, nonce);
                }
                match uninstall_equip(&equip) {
                    Ok(()) => {
                        drop(guard);
                        self.rig_locks.unregister(&equip).await;
                        purview_remove_rig(&equip);
                        success(nonce, &EquipResult { summary: None })
                    }
                    Err(e) => error_to_call_result(e, nonce),
                }
            }
        }
    }
}

/// Bring a freshly installed rig into view.
fn purview_add_rig(
    rig: &str,
    current_ids: &[String],
) {
    let _repo = equip_repo_lock();
    let mut rows = match load_purviews() {
        Ok(rows) => rows.unwrap_or_default(),
        Err(e) => {
            eprintln!("grammar: purview not updated for {rig}: {e:?}");
            return;
        }
    };
    let builtin = builtin_name(rig);
    let pattern = match builtin {
        Some(name) => {
            seed_builtin(&mut rows, name);
            format!("{PURVIEW_REF}{PURVIEW_CORE}")
        }
        None => format!("{rig}:"),
    };
    let mut targets: Vec<String> = vec![PURVIEW_DEFAULT.to_string()];
    for id in current_ids {
        if id != PURVIEW_ALL && !targets.contains(id) {
            targets.push(id.clone());
        }
    }
    for id in targets {
        if builtin.is_some() && (id == PURVIEW_CORE || builtin_equip(&id).is_some()) {
            continue;
        }
        if let Some(row) = rows.iter_mut().find(|row| row.id == id) {
            let present = row.namepath_patterns.contains(&pattern)
                || row.namepath_patterns.iter().any(|p| is_rig_pattern(p, rig));
            if !present {
                row.namepath_patterns.push(pattern.clone());
            }
        }
    }
    prune_dangling(&mut rows);
    if let Err(e) = save_purviews(&rows) {
        eprintln!("grammar: purview not updated for {rig}: {e:?}");
    }
}

/// Drop an uninstalled rig from every purview that named it.
fn purview_remove_rig(rig: &str) {
    let _repo = equip_repo_lock();
    let mut rows = match load_purviews() {
        Ok(rows) => rows.unwrap_or_default(),
        Err(e) => {
            eprintln!("grammar: purview not updated for {rig}: {e:?}");
            return;
        }
    };
    for row in rows.iter_mut() {
        row.namepath_patterns.retain(|p| !is_rig_pattern(p, rig));
    }
    prune_dangling(&mut rows);
    if let Err(e) = save_purviews(&rows) {
        eprintln!("grammar: purview not updated for {rig}: {e:?}");
    }
}

fn check_summary_from(result: &ValidationResult) -> CheckSummary {
    let (errors, warnings) = Diagnostic::bucket(result.diagnostics.clone());
    CheckSummary {
        ok: errors.is_empty(),
        errors,
        warnings,
    }
}
