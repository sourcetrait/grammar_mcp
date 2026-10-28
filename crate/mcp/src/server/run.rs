use crate::*;

/// Serve MCP over stdio until the client leaves; the error says why it could not.
pub(crate) async fn run_server() -> Result<(), String> {
    install_child_subreaper();
    ensure_cert_profile();
    ensure_substrate()
        .await
        .map_err(|e| format!("the equip substrate: {e}"))?;
    let (nonce_gen, mcp_nom) = process_identity();
    let system = Arc::new(start_system(&mcp_nom, true).await?);
    let server = NuSh::new(nonce_gen, mcp_nom, system.clone(), &config().deny);
    spawn_signal_sweep(system.clone());
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

/// RFC 6455 "going away" - a server shutting down, which is exactly this.
const CLOSE_GOING_AWAY: u16 = 1001;
const SHUTDOWN_REASON: &str = "host shutting down";

/// How long a shutdown waits for the close frame to reach the wire.
const CLOSE_FLUSH_GRACE: tk::TkDuration = tk::TkDuration::from_millis(500);

/// Tell the channel's peer we are going away, and wait for the flush.
async fn close_channel_for_shutdown() {
    if let Some(done) = channel_handle().close_and_await(CLOSE_GOING_AWAY, SHUTDOWN_REASON) {
        let _ = tk::timeout(CLOSE_FLUSH_GRACE, done).await;
    }
}

/// Sweep on TERM / INT / HUP, then exit with the conventional `128 + signo`.
///
/// The scratch directory goes last: the exit skips the ordinary end's removal.
fn spawn_signal_sweep(system: Arc<McpHandle>) {
    tk::spawn(async move {
        let mut term = match tk::signal(tk::SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("grammar: SIGTERM handler not installed: {e}");
                return;
            }
        };
        let mut interrupt = match tk::signal(tk::SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("grammar: SIGINT handler not installed: {e}");
                return;
            }
        };
        let mut hangup = match tk::signal(tk::SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("grammar: SIGHUP handler not installed: {e}");
                return;
            }
        };
        let signo = tokio::select! {
            _ = term.recv() => nix::sys::signal::Signal::SIGTERM as i32,
            _ = interrupt.recv() => nix::sys::signal::Signal::SIGINT as i32,
            _ = hangup.recv() => nix::sys::signal::Signal::SIGHUP as i32,
        };
        close_channel_for_shutdown().await;
        system.shutdown().await;
        remove_scratch_dirs();
        process::exit(128 + signo);
    });
}
