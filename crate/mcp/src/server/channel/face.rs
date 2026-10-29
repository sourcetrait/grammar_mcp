//! The Channel's and the remote channels' commands, served by either server.
use crate::*;

/// The Channel's face to a server: its handle, open lock, identity, and links.
pub(crate) struct ChannelFace {
    /// The host's single packet channel; present but CLOSED at startup.
    pub(crate) channel: Arc<ChannelHandle>,
    /// Serializes `channel_open`'s decide-then-start across its await.
    pub(crate) channel_open_lock: Arc<tk::AsyncMutex<()>>,
    pub(crate) nonce_gen: Arc<datum::NonceGenerator>,
    /// This host process's identity, minted once at startup.
    pub(crate) mcp_nom: datum::NomPair,
    /// The process-wide registry of established remote links, keyed by alias.
    pub(crate) remote_links: Arc<std::sync::Mutex<HashMap<String, RemoteLinkEntry>>>,
}

impl ChannelFace {
    pub(crate) fn new(nonce_gen: Arc<datum::NonceGenerator>, mcp_nom: datum::NomPair) -> Self {
        Self {
            channel: channel_handle(),
            channel_open_lock: Arc::new(tk::AsyncMutex::new(())),
            nonce_gen,
            mcp_nom,
            remote_links: remote_links(),
        }
    }
}

/// RFC 6455 "going away" - a server shutting down, which is exactly this.
const CLOSE_GOING_AWAY: u16 = 1001;
const SHUTDOWN_REASON: &str = "host shutting down";

/// How long a shutdown waits for the close frame to reach the wire.
const CLOSE_FLUSH_GRACE: tk::TkDuration = tk::TkDuration::from_millis(500);

/// Tell the channel's peer we are going away, and wait for the flush.
pub(crate) async fn close_channel_for_shutdown() {
    if let Some(done) = channel_handle().close_and_await(CLOSE_GOING_AWAY, SHUTDOWN_REASON) {
        let _ = tk::timeout(CLOSE_FLUSH_GRACE, done).await;
    }
}

/// Sweep on TERM, INT, or HUP, then exit with the conventional `128 + signo`.
///
/// The Channel closes first, then the server's own shutdown runs.
///
/// The scratch directory goes last: the exit skips the ordinary end's removal.
#[cfg(unix)]
pub(crate) fn spawn_signal_sweep<F, Fut>(shutdown: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
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
            _ = term.recv() => tk::SignalKind::terminate().as_raw_value(),
            _ = interrupt.recv() => tk::SignalKind::interrupt().as_raw_value(),
            _ = hangup.recv() => tk::SignalKind::hangup().as_raw_value(),
        };
        close_channel_for_shutdown().await;
        shutdown().await;
        remove_scratch_dirs();
        process::exit(128 + signo);
    });
}

/// Off Unix no signal is swept; the ordinary end closes and cleans up.
#[cfg(not(unix))]
pub(crate) fn spawn_signal_sweep<F, Fut>(_shutdown: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
}
