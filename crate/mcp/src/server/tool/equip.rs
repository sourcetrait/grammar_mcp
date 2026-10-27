use crate::*;

/// Parameters for each `interface equip` action on a source.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct EquipParams {
    /// The equip's source: an `equip/<author>/<name>` directory.
    pub source: String,
}

/// `new` summary: the paths scaffolded for the fresh equip's rig.
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewSummary {
    pub created: Vec<String>,
}

/// The per-action summary, `install`'s its commit's; `uninstall` returns none.
#[derive(Debug, ser::Serialize)]
#[serde(untagged)]
pub(crate) enum EquipSummary {
    New(NewSummary),
    Install(CommitResult),
    Check(CheckSummary),
}

/// Success result of each `interface equip` action on a source.
#[derive(Debug, ser::Serialize)]
pub(crate) struct EquipResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<EquipSummary>,
}

impl NuSh {
    /// `interface equip new`: register an equip at its source and seed its rig.
    pub(crate) async fn equip_new(
        &self,
        nonce: datum::Nonce,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.engine.control(EngineCommand::EquipNew { source: p.source }).await {
            Ok(ControlAnswer::Created(created)) => success(
                nonce,
                &EquipResult {
                    summary: Some(EquipSummary::New(NewSummary { created })),
                },
            ),
            Ok(_) => error_to_call_result(unexpected("equip_new"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }

    /// `interface equip install`: validate an equip's whole source and install it.
    pub(crate) async fn equip_install(
        &self,
        nonce: datum::Nonce,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.engine.control(EngineCommand::EquipInstall { source: p.source }).await {
            Ok(ControlAnswer::Installed { equip, result }) => {
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
            Ok(_) => error_to_call_result(unexpected("equip_install"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }

    /// `interface equip check`: validate an installed equip's source, writing nothing.
    pub(crate) async fn equip_check(
        &self,
        nonce: datum::Nonce,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.engine.control(EngineCommand::EquipCheck { source: p.source }).await {
            Ok(ControlAnswer::Checked(summary)) => success(
                nonce,
                &EquipResult {
                    summary: Some(EquipSummary::Check(summary)),
                },
            ),
            Ok(_) => error_to_call_result(unexpected("equip_check"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }

    /// `interface equip uninstall`: remove an installed equip; its source stays.
    pub(crate) async fn equip_uninstall(
        &self,
        nonce: datum::Nonce,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.engine.control(EngineCommand::EquipUninstall { source: p.source }).await {
            Ok(ControlAnswer::Uninstalled { equip }) => {
                if let Some(equip) = equip
                    && let Some(rows) = purview_remove_rig(&equip)
                {
                    self.current_purview.retain_known(Some(&rows));
                }
                success(nonce, &EquipResult { summary: None })
            }
            Ok(_) => error_to_call_result(unexpected("equip_uninstall"), nonce),
            Err(e) => error_to_call_result(e, nonce),
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

/// Drop an uninstalled rig from every purview that named it; the rows saved.
fn purview_remove_rig(rig: &str) -> Option<Vec<PurviewRow>> {
    let _repo = equip_repo_lock();
    let mut rows = match load_purviews() {
        Ok(rows) => rows.unwrap_or_default(),
        Err(e) => {
            eprintln!("grammar: purview not updated for {rig}: {e:?}");
            return None;
        }
    };
    for row in rows.iter_mut() {
        row.namepath_patterns.retain(|p| !is_rig_pattern(p, rig));
    }
    prune_dangling(&mut rows);
    if let Err(e) = save_purviews(&rows) {
        eprintln!("grammar: purview not updated for {rig}: {e:?}");
        return None;
    }
    Some(rows)
}
