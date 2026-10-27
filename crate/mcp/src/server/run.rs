use crate::*;

/// Serve MCP over stdio until the client leaves; the error says why it could not.
pub(crate) async fn run_server() -> Result<(), String> {
    install_child_subreaper();
    ensure_cert_profile();
    ensure_substrate()
        .await
        .map_err(|e| format!("the equip substrate: {e}"))?;
    let engine = Arc::new(EngineHandle::start().await.map_err(|e| e.to_string())?);
    let nonce_gen = Arc::new(datum::NonceGenerator::new());
    let server = NuSh::new(nonce_gen, engine.clone(), &config().deny);
    let (emergency_tx, emergency_rx) = tk::unbounded_channel::<Emergency>();
    spawn_emergency_responder(emergency_rx, server.mcp_nom.as_str().to_string());
    server.channel.install_emergency(emergency_tx.clone());
    spawn_watchdog(WatchdogDeps {
        engine: engine.clone(),
        tx: emergency_tx,
    });
    let _host_lock = match acquire_host_lock(server.mcp_nom.as_str()) {
        Ok(lock) => Some(lock),
        Err(e) => {
            eprintln!("grammar: host lock not held: {e}");
            None
        }
    };
    spawn_signal_sweep(engine.clone());
    let service = server
        .serve(mcp::stdio())
        .await
        .map_err(|e| format!("serve stdio: {e}"))?;
    let served = service.waiting().await;
    close_channel_for_shutdown().await;
    teardown(&engine).await;
    served
        .map(|_| ())
        .map_err(|e| format!("the stdio service: {e}"))
}

/// Cancel every call in flight and reap its tree, the engine's jobs and plugins.
async fn teardown(engine: &EngineHandle) {
    if let Err(e) = engine.control(EngineCommand::Teardown).await {
        eprintln!("grammar: the engine's teardown: {e}");
    }
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
fn spawn_signal_sweep(engine: Arc<EngineHandle>) {
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
        teardown(&engine).await;
        process::exit(128 + signo);
    });
}
