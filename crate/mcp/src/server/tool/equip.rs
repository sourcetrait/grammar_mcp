use crate::*;

/// Parameters for each `interface equip` action on a source.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct EquipParams {
    /// The equip's source: an `equip/<author>/<name>` directory.
    pub source: String,
}

/// `new` summary: the paths scaffolded for the fresh equip's rig.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewSummary {
    pub created: Vec<String>,
}

/// The per-action summary, `install`'s its commit's; `uninstall` returns none.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
#[serde(untagged)]
pub(crate) enum EquipSummary {
    New(NewSummary),
    Install(CommitResult),
    Check(CheckSummary),
}

/// Success result of each `interface equip` action on a source.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct EquipResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<EquipSummary>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface equip new`: register an equip at its source and seed its rig.
    pub(crate) async fn equip_new(
        &self,
        nonce: datum::Nonce,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.system.control(EngineCommand::EquipNew { source: p.source }).await {
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
        match self.system.control(EngineCommand::EquipInstall { source: p.source }).await {
            Ok(ControlAnswer::Installed { equip, result }) => {
                self.purview_rig(equip, true).await;
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
        match self.system.control(EngineCommand::EquipCheck { source: p.source }).await {
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
        match self.system.control(EngineCommand::EquipUninstall { source: p.source }).await {
            Ok(ControlAnswer::Uninstalled { equip }) => {
                if let Some(equip) = equip {
                    self.purview_rig(equip, false).await;
                }
                success(nonce, &EquipResult { summary: None })
            }
            Ok(_) => error_to_call_result(unexpected("equip_uninstall"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// A rig installed or uninstalled: the engine's purviews adjusted, the view kept.
    async fn purview_rig(
        &self,
        rig: String,
        installed: bool,
    ) {
        let adjust = PurviewRigRequest {
            rig: rig.clone(),
            installed,
            current: self.current_purview.ids(),
        };
        match self.system.request(adjust).await {
            Ok(Ok(rows)) if !installed => self.current_purview.retain_known(&rows),
            Ok(Ok(_)) => {}
            Ok(Err(e)) => eprintln!("grammar: purview not updated for {rig}: {e:?}"),
            Err(e) => eprintln!("grammar: purview not updated for {rig}: {e:?}"),
        }
    }
}
