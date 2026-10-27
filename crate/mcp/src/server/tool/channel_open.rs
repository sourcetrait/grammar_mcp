use crate::*;

/// Parameters for `interface channel open`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ChannelOpenParams {
    /// `sync` or `async`: how the Channel's packets reach the agent.
    pub mode: ChannelMode,
}

/// Success result of `interface channel open`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct ChannelOpenResult {
    /// `new` when this call opened the channel, `existing` when one was open.
    pub status: String,
    /// The mode the channel is open in.
    pub mode: ChannelMode,
    /// The endpoint to point a Monitor at, its port the bound one; null in sync.
    pub wss: Option<String>,
    /// The directory an attachment lands in.
    pub inbox: String,
}

const STATUS_NEW: &str = "new";
const STATUS_EXISTING: &str = "existing";

/// How long a peer has to prove it owns the stdio session.
const VERIFY_WINDOW: tk::TkDuration = tk::TkDuration::from_secs(300);

/// RFC 6455 policy violation - the peer never verified.
const CLOSE_UNVERIFIED: u16 = 1008;
const UNVERIFIED_REASON: &str = "verification window expired";

/// `$XDGX_SHM_DIR/mcp/<mcp_nom>/inbox`, created here.
fn ensure_inbox(
    channel: &ChannelHandle,
    mcp_nom: &datum::NomPair,
) -> Result<String, GrammarMcpError> {
    let dir = inbox_dir(mcp_nom).map_err(|reason| GrammarMcpError::ChannelStart { reason })?;
    fs::create_dir_all(&dir)?;
    channel.set_inbox(dir.clone());
    Ok(dir.display().to_string())
}

/// The host's `channel/Open` packet, rendered.
fn greeting(
    nonce_gen: &datum::NonceGenerator,
    mcp_nom: &datum::NomPair,
) -> Result<String, GrammarMcpError> {
    open_packet(nonce_gen, mcp_nom).map_err(|reason| GrammarMcpError::Internal {
        phase: "channel_open::render".to_string(),
        reason,
    })
}

/// Re-greet an existing channel's peer.
fn resend_open_packet(
    channel: &ChannelHandle,
    nonce_gen: &datum::NonceGenerator,
    mcp_nom: &datum::NomPair,
) -> Result<(), GrammarMcpError> {
    let line = greeting(nonce_gen, mcp_nom)?;
    channel.send_control(line).map_err(|e| match e {
        ChannelSendError::NotOpen => GrammarMcpError::ChannelNotOpen,
        _ => GrammarMcpError::ChannelPeerGone,
    })
}

/// Start the verify window.
fn arm_verify_timer(channel: Arc<ChannelHandle>) {
    let (cancel_tx, cancel_rx) = tk::oneshot::channel::<()>();
    channel.arm_verify(cancel_tx);
    tk::spawn(async move {
        tokio::select! {
            _ = cancel_rx => {}
            _ = tk::sleep(VERIFY_WINDOW) => {
                channel.close_if_unverified(CLOSE_UNVERIFIED, UNVERIFIED_REASON);
            }
        }
    });
}

impl NuSh {
    /// `interface channel open`: open the packet channel in its mode.
    pub(crate) async fn channel_open(
        &self,
        nonce: datum::Nonce,
        p: ChannelOpenParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let inbox = match ensure_inbox(&self.channel, &self.mcp_nom) {
            Ok(path) => path,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let _opening = self.channel_open_lock.lock().await;
        let opened = match self.open_or_report(p.mode).await {
            Ok(opened) => opened,
            Err(error) => return error_to_call_result(error, nonce),
        };
        arm_verify_timer(self.channel.clone());
        let (label, wss) = opened;
        success(
            nonce,
            &ChannelOpenResult {
                status: label.to_string(),
                mode: p.mode,
                wss,
                inbox,
            },
        )
    }

    /// Open a closed channel in `mode`, or report the open one in the same mode.
    async fn open_or_report(
        &self,
        mode: ChannelMode,
    ) -> Result<(&'static str, Option<String>), GrammarMcpError> {
        let status = self.channel.status();
        if matches!(status.phase, ChannelPhase::Closed) {
            return match mode {
                ChannelMode::Async => {
                    let url =
                        start_channel_hub(&self.channel, &self.mcp_nom, self.nonce_gen.clone())
                            .await?;
                    Ok((STATUS_NEW, Some(url)))
                }
                ChannelMode::Sync => {
                    self.channel
                        .install_sync(greeting(&self.nonce_gen, &self.mcp_nom)?);
                    Ok((STATUS_NEW, None))
                }
            };
        }
        let open = status.mode.ok_or_else(|| GrammarMcpError::Internal {
            phase: "channel_open::mode".to_string(),
            reason: "an open channel carries no mode".to_string(),
        })?;
        if open != mode {
            return Err(GrammarMcpError::ChannelWrongMode { open, asked: mode });
        }
        if open == ChannelMode::Async && status.url.is_none() {
            return Err(GrammarMcpError::Internal {
                phase: "channel_open::url".to_string(),
                reason: "an open channel carries no url".to_string(),
            });
        }
        if status.claimed {
            resend_open_packet(&self.channel, &self.nonce_gen, &self.mcp_nom)?;
        }
        Ok((STATUS_EXISTING, status.url))
    }
}
