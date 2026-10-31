use crate::*;

/// The test driver's server: MCP over stdio on the in-process engine.
pub(crate) async fn run_server(config: Arc<Config>) -> Result<(), String> {
    install_child_subreaper();
    ensure_cert_profile();
    ensure_substrate()
        .await
        .map_err(|e| format!("the equip substrate: {e}"))?;
    let controller = McpController::embedded(config.clone(), true).await?;
    controller.sweep_signals();
    let server = NuSh::new(controller.clone(), &config.deny);
    let service = server
        .serve(mcp::stdio())
        .await
        .map_err(|e| format!("serve stdio: {e}"))?;
    let served = service.waiting().await;
    controller.shutdown().await;
    served
        .map(|_| ())
        .map_err(|e| format!("the stdio service: {e}"))
}
