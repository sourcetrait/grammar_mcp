use crate::*;

/// The test driver's server: MCP over stdio on the in-process engine.
///
/// Never the serving path; it runs until the client leaves, and the error says
/// why it could not.
pub(crate) async fn run_server() -> Result<(), String> {
    install_child_subreaper();
    ensure_cert_profile();
    ensure_substrate()
        .await
        .map_err(|e| format!("the equip substrate: {e}"))?;
    let (nonce_gen, mcp_nom) = process_identity();
    let system = Arc::new(start_system(&mcp_nom, true).await?);
    let server = NuSh::new(nonce_gen, mcp_nom, system.clone(), &config().deny);
    let swept = system.clone();
    spawn_signal_sweep(move || async move { swept.shutdown().await });
    let service = server
        .serve(mcp::stdio())
        .await
        .map_err(|e| format!("serve stdio: {e}"))?;
    let served = service.waiting().await;
    close_channel_for_shutdown().await;
    system.shutdown().await;
    served
        .map(|_| ())
        .map_err(|e| format!("the stdio service: {e}"))
}
