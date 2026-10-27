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
    /// This host process's id, stable for its lifetime.
    pub mcp_nom: String,
    /// The agent working directory, exported as $env.EQUIP_WORK_DIR.
    pub work_dir: String,
    pub plugins: Vec<crate::plugins::PluginInfo>,
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
        let rows = match load_purviews() {
            Ok(rows) => rows,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let ids = self.current_purview.ids();
        let patterns = parse_patterns(&expand_values(
            &resolve_patterns(&ids, rows.as_ref()),
            rows.as_ref(),
        ));
        success(
            nonce,
            &InfoResult {
                name: consts::GRAMMAR.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                nu_version: env!("NU_VERSION").to_string(),
                id: config().id.clone(),
                namespace: config().namespace.clone(),
                mcp_nom: self.mcp_nom.as_str().to_string(),
                work_dir: config().work_dir.display().to_string(),
                plugins: list_registered_plugins(),
                signatures: render_signatures_within(&self.rig_locks, &patterns).await,
                purview: purview_views(&ids, rows.as_ref()),
            },
        )
    }
}
