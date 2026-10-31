use crate::*;

/// Success result of `interface info`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct InfoResult {
    pub name: String,
    pub version: String,
    pub nu_version: String,
    /// The `--id` this server was configured with.
    pub id: String,
    pub namespace: String,
    /// This host process's nom, stable for its life: $env.EQUIP_MCP_NOM.
    pub nom: String,
    /// The agent working directory, exported as $env.EQUIP_WORK_DIR.
    pub work_dir: String,
    /// The host's bulk scratch directory, exported as $env.EQUIP_TMP.
    pub tmp: String,
    /// The host's transient scratch directory, exported as $env.EQUIP_TRANSIENT.
    pub transient: String,
    pub plugins: Vec<PluginInfo>,
    /// The callable surface within the reported purview, as one block.
    pub signatures: String,
    /// The purview `signatures` was rendered for.
    pub purview: Vec<PurviewView>,
}

impl NuSh {
    /// `interface info`: versions, plugins, and every rig's callable signatures.
    pub(crate) async fn info(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let view = match self.resolve_purviews(self.current_purview.ids()).await {
            Ok(view) => view,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let signatures = match self.signatures(view.patterns).await {
            Ok(signatures) => signatures,
            Err(error) => return error_to_call_result(error, nonce),
        };
        success(
            nonce,
            &InfoResult {
                name: consts::GRAMMAR.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                nu_version: NU_VERSION.to_string(),
                id: self.controller.config.id.clone(),
                namespace: self.controller.config.namespace.clone(),
                nom: self.mcp_nom.as_str().to_string(),
                work_dir: self.controller.config.work_dir.display().to_string(),
                tmp: tmp_dir().map(|dir| dir.display().to_string()).unwrap_or_default(),
                transient: transient_dir().map(|dir| dir.display().to_string()).unwrap_or_default(),
                plugins: list_registered_plugins(),
                signatures,
                purview: view.views,
            },
        )
    }
}
