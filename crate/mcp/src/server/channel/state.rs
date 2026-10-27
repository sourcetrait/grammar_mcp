use crate::*;

/// The model-path prefix RESERVED for host-originated packets.
pub(crate) const MCP_RESERVED_PREFIX: &str = "mcp/";

/// The usable frame maximum.
pub(crate) const MAX_FRAME_BYTES: usize = 1_048_575;

/// The env var naming the tmpfs IPC root the inbox lives under.
pub(crate) const SHM_ROOT_VAR: &str = "$XDGX_SHM_DIR";

/// `<shm>/mcp/<mcp_nom>/inbox`, where attachments and a link's landed files meet.
pub(crate) fn inbox_dir(mcp_nom: &datum::NomPair) -> Result<PathBuf, String> {
    Ok(expand_path(Path::new(SHM_ROOT_VAR))?
        .join("mcp")
        .join(mcp_nom.as_str())
        .join("inbox"))
}

/// What an emit is allowed to do right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChannelPhase {
    /// No hub running; the ordinary state of a session that never opens one.
    Closed,
    /// The hub is up but the peer has not yet proven it owns the session.
    Open,
    /// `interface channel ready` succeeded; emits flow.
    Verified,
}

/// How an open Channel's packets reach its peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ser::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelMode {
    /// A Monitor reads them from the hub's loopback WSS stream as they come.
    Async,
    /// They queue until the agent takes them by `interface channel sync`.
    Sync,
}

impl ChannelMode {
    /// The mode's name, as `interface channel open` takes it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Async => "async",
            Self::Sync => "sync",
        }
    }

    /// The mode a name names, if any.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "async" => Some(Self::Async),
            "sync" => Some(Self::Sync),
            _ => None,
        }
    }
}

/// The most packets a sync Channel queues; past it, the oldest is dropped.
pub(crate) const SYNC_QUEUE_CAP: usize = 1000;

/// What a planned close carries: the code, the reason, and an optional ack.
pub(crate) type CloseSignal = (u16, String, Option<tk::oneshot::Sender<()>>);

/// A snapshot a caller can hold without keeping the lock.
#[derive(Clone, Debug)]
pub(crate) struct ChannelStatus {
    pub phase: ChannelPhase,
    pub mode: Option<ChannelMode>,
    pub url: Option<String>,
    pub claimed: bool,
}

/// Where an emitted line goes: the hub's queue, or the queue a sync takes.
enum PacketSink {
    /// The hub's queue, which its peer task writes to the socket.
    Hub(tk::UnboundedSender<String>),
    /// The lines a sync takes, and how many the cap dropped since the last.
    Sync {
        lines: std::collections::VecDeque<String>,
        dropped: u64,
    },
}

/// What the spam counter decided about one send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpamVerdict {
    /// Under both thresholds.
    Clear,
    /// Crossed the soft threshold for the FIRST time.
    Warn { hits: u32, window_secs: u64 },
    /// Crossed the hard threshold; refuse the send and stop the offender.
    Stop {
        hits: u32,
        window_secs: u64,
        notify: bool,
    },
}

/// One origin's recent sends, plus its warned and stopped flags.
#[derive(Default)]
struct OriginCounter {
    hits: Vec<Instant>,
    warned: bool,
    stopped: bool,
}

struct ChannelInner {
    phase: ChannelPhase,
    /// Set at an open, cleared at a close.
    mode: Option<ChannelMode>,
    url: Option<String>,
    /// The live spam policy, seeded from CONFIG at construction.
    spam: SpamThresholds,
    /// Recent sends per origin, bounded by eviction rather than by a cap.
    counters: HashMap<String, OriginCounter>,
    /// Installed by `run_server`; absent on the one-shot CLI path.
    emergency: Option<EmergencyTx>,
    /// Where rendered, newline-escaped NUON lines go, one per packet.
    packets: Option<PacketSink>,
    /// The planned-close signal, deliberately NOT sharing the packet queue.
    close: Option<tk::oneshot::Sender<CloseSignal>>,
    shutdown: Option<tk::oneshot::Sender<()>>,
    claimed: Option<Arc<AtomicBool>>,
    /// Holding this sender keeps the verify timer armed; DROPPING it cancels.
    verify_cancel: Option<tk::oneshot::Sender<()>>,
    /// The inbox directory, set at open; a deliberate close prunes it.
    inbox: Option<PathBuf>,
}

/// Why an emit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelSendError {
    /// No channel exists. Terminal for the caller's purposes.
    NotOpen,
    /// Open but unverified - forbidden, and the channel is torn down for trying.
    NotVerified,
    /// The hub task is gone while state still said otherwise. Terminal.
    HubGone,
}

impl ChannelSendError {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NotOpen => "the channel is not open; run `interface channel open` first",
            Self::NotVerified => {
                "the channel is not verified; emitting before `interface channel ready` is \
                 forbidden"
            }
            Self::HubGone => "the channel hub is gone",
        }
    }
}

/// Why a verification was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelVerifyError {
    /// Nothing to verify - `interface channel open` has not run.
    NotOpen,
    /// Open, but nothing has claimed it in its mode, so there is no claim to prove.
    NotClaimed(ChannelMode),
}

/// Why a sync was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelSyncError {
    /// No channel exists.
    NotOpen,
    /// The channel is open in the other mode, named here.
    WrongMode(ChannelMode),
}

/// The host's single channel, guarded by a std Mutex.
pub(crate) struct ChannelHandle {
    inner: std::sync::Mutex<ChannelInner>,
    /// Mints every `MsgId` on this channel, so all ids share one counter.
    nonce_gen: Arc<datum::NonceGenerator>,
}

impl ChannelHandle {
    /// Takes its starting policy rather than reading CONFIG.
    pub(crate) fn new(spam: SpamThresholds) -> Self {
        Self {
            inner: std::sync::Mutex::new(ChannelInner {
                phase: ChannelPhase::Closed,
                mode: None,
                url: None,
                spam,
                counters: HashMap::new(),
                emergency: None,
                packets: None,
                close: None,
                shutdown: None,
                claimed: None,
                verify_cancel: None,
                inbox: None,
            }),
            nonce_gen: Arc::new(datum::NonceGenerator::new()),
        }
    }

    pub(crate) fn nonce_gen(&self) -> &datum::NonceGenerator {
        &self.nonce_gen
    }

    /// Where attachments land, set by `channel_open`.
    pub(crate) fn set_inbox(
        &self,
        dir: PathBuf,
    ) {
        self.lock().inbox = Some(dir);
    }

    pub(crate) fn inbox(&self) -> Option<PathBuf> {
        self.lock().inbox.clone()
    }

    /// Take the inbox path, clearing it - the prune handoff for a close.
    pub(crate) fn take_inbox(&self) -> Option<PathBuf> {
        self.lock().inbox.take()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ChannelInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn status(&self) -> ChannelStatus {
        let inner = self.lock();
        ChannelStatus {
            phase: inner.phase,
            mode: inner.mode,
            url: inner.url.clone(),
            claimed: claimed_of(&inner),
        }
    }

    /// Record a freshly bound hub; the phase moves to Open, the mode async.
    pub(crate) fn install(
        &self,
        url: String,
        packets: tk::UnboundedSender<String>,
        close: tk::oneshot::Sender<CloseSignal>,
        shutdown: tk::oneshot::Sender<()>,
        claimed: Arc<AtomicBool>,
    ) {
        let mut inner = self.lock();
        inner.phase = ChannelPhase::Open;
        inner.mode = Some(ChannelMode::Async);
        inner.url = Some(url);
        inner.packets = Some(PacketSink::Hub(packets));
        inner.close = Some(close);
        inner.shutdown = Some(shutdown);
        inner.claimed = Some(claimed);
    }

    /// Open a sync Channel, its greeting the first line queued, its claim unset.
    pub(crate) fn install_sync(
        &self,
        open_line: String,
    ) {
        let mut inner = self.lock();
        inner.phase = ChannelPhase::Open;
        inner.mode = Some(ChannelMode::Sync);
        inner.url = None;
        inner.packets = Some(PacketSink::Sync {
            lines: std::collections::VecDeque::from([open_line]),
            dropped: 0,
        });
        inner.close = None;
        inner.shutdown = None;
        inner.claimed = Some(Arc::new(AtomicBool::new(false)));
    }

    /// Take a sync Channel's queued lines and its drop count, claiming it.
    pub(crate) fn take_sync(&self) -> Result<(Vec<String>, u64), ChannelSyncError> {
        let mut guard = self.lock();
        let inner = &mut *guard;
        if matches!(inner.phase, ChannelPhase::Closed) {
            return Err(ChannelSyncError::NotOpen);
        }
        let taken = match &mut inner.packets {
            Some(PacketSink::Sync { lines, dropped }) => {
                (lines.drain(..).collect(), std::mem::take(dropped))
            }
            _ => {
                return Err(ChannelSyncError::WrongMode(
                    inner.mode.unwrap_or(ChannelMode::Async),
                ));
            }
        };
        if let Some(claimed) = &inner.claimed {
            claimed.store(true, Ordering::SeqCst);
        }
        Ok(taken)
    }

    /// Arm the verify timer, standing any previous one down.
    pub(crate) fn arm_verify(
        &self,
        cancel: tk::oneshot::Sender<()>,
    ) {
        self.lock().verify_cancel = Some(cancel);
    }

    /// Promote Open to Verified, but ONLY once a peer has actually claimed.
    pub(crate) fn mark_verified(&self) -> Result<(), ChannelVerifyError> {
        let mut inner = self.lock();
        if matches!(inner.phase, ChannelPhase::Closed) {
            return Err(ChannelVerifyError::NotOpen);
        }
        if !claimed_of(&inner) {
            return Err(ChannelVerifyError::NotClaimed(
                inner.mode.unwrap_or(ChannelMode::Async),
            ));
        }
        inner.phase = ChannelPhase::Verified;
        inner.verify_cancel = None;
        Ok(())
    }

    /// Tear the channel down: ask the peer to close with a reason, then drop it.
    pub(crate) fn close(
        &self,
        code: u16,
        reason: &str,
    ) -> bool {
        let mut inner = self.lock();
        if matches!(inner.phase, ChannelPhase::Closed) {
            return false;
        }
        close_locked(&mut inner, code, reason, None);
        true
    }

    /// Close, handing back a receiver that fires once the frame is on the wire.
    pub(crate) fn close_and_await(
        &self,
        code: u16,
        reason: &str,
    ) -> Option<tk::oneshot::Receiver<()>> {
        let mut inner = self.lock();
        if matches!(inner.phase, ChannelPhase::Closed) {
            return None;
        }
        let (done_tx, done_rx) = tk::oneshot::channel::<()>();
        close_locked(&mut inner, code, reason, Some(done_tx));
        Some(done_rx)
    }

    /// Close ONLY while still unverified - the verify timer's expiry action.
    pub(crate) fn close_if_unverified(
        &self,
        code: u16,
        reason: &str,
    ) -> bool {
        let mut inner = self.lock();
        if !matches!(inner.phase, ChannelPhase::Open) {
            return false;
        }
        close_locked(&mut inner, code, reason, None);
        true
    }

    /// Push a HOST control packet, bypassing the verification gate.
    pub(crate) fn send_control(
        &self,
        line: String,
    ) -> Result<(), ChannelSendError> {
        let mut inner = self.lock();
        if matches!(inner.phase, ChannelPhase::Closed) {
            return Err(ChannelSendError::NotOpen);
        }
        push_locked(&mut inner, line)
    }

    /// The emit path; the phase is re-read here, at the moment of sending.
    pub(crate) fn emit(
        &self,
        line: String,
    ) -> Result<(), ChannelSendError> {
        let mut inner = self.lock();
        match inner.phase {
            ChannelPhase::Closed => Err(ChannelSendError::NotOpen),
            ChannelPhase::Open => Err(ChannelSendError::NotVerified),
            ChannelPhase::Verified => push_locked(&mut inner, line),
        }
    }
}

impl ChannelHandle {
    /// Install the emergency lane.
    pub(crate) fn install_emergency(
        &self,
        tx: EmergencyTx,
    ) {
        self.lock().emergency = Some(tx);
    }

    pub(crate) fn thresholds(&self) -> SpamThresholds {
        self.lock().spam
    }

    /// Apply a PARTIAL update and return what is now in force.
    pub(crate) fn set_thresholds(
        &self,
        warn_window_secs: Option<u64>,
        warn_rate: Option<u32>,
        error_window_secs: Option<u64>,
        error_rate: Option<u32>,
    ) -> Result<SpamThresholds, String> {
        let mut inner = self.lock();
        let mut spam = inner.spam;
        if let Some(secs) = warn_window_secs {
            spam.warn_window = positive_window("spam_warn_window_secs", secs)?;
        }
        if let Some(rate) = warn_rate {
            spam.warn_rate = positive_rate("spam_warn_rate", rate)?;
        }
        if let Some(secs) = error_window_secs {
            spam.error_window = positive_window("spam_error_window_secs", secs)?;
        }
        if let Some(rate) = error_rate {
            spam.error_rate = positive_rate("spam_error_rate", rate)?;
        }
        inner.spam = spam;
        Ok(spam)
    }

    /// Record one send by `from` and say what it costs.
    pub(crate) fn record_send(
        &self,
        from: &str,
    ) -> SpamVerdict {
        self.record_send_at(from, Instant::now())
    }

    /// The counting itself, with `now` injected so it is testable.
    pub(crate) fn record_send_at(
        &self,
        from: &str,
        now: Instant,
    ) -> SpamVerdict {
        let mut inner = self.lock();
        let spam = inner.spam;
        let retention = spam.retention();
        inner.counters.retain(|_, counter| {
            counter
                .hits
                .retain(|at| now.saturating_duration_since(*at) < retention);
            !counter.hits.is_empty()
        });
        let counter = inner.counters.entry(from.to_string()).or_default();
        counter.hits.push(now);
        let within = |window: tk::TkDuration| -> u32 {
            counter
                .hits
                .iter()
                .filter(|at| now.saturating_duration_since(**at) < window)
                .count() as u32
        };
        let errors = within(spam.error_window);
        if errors >= spam.error_rate {
            let notify = !counter.stopped;
            counter.stopped = true;
            return SpamVerdict::Stop {
                hits: errors,
                window_secs: spam.error_window.as_secs(),
                notify,
            };
        }
        let warns = within(spam.warn_window);
        if warns >= spam.warn_rate && !counter.warned {
            counter.warned = true;
            return SpamVerdict::Warn {
                hits: warns,
                window_secs: spam.warn_window.as_secs(),
            };
        }
        SpamVerdict::Clear
    }

    /// Push an Emergency onto the internal lane, if one is installed.
    pub(crate) fn fire_emergency(
        &self,
        emergency: Emergency,
    ) {
        if let Some(tx) = &self.lock().emergency {
            let _ = tx.send(emergency);
        }
    }
}

fn positive_window(
    name: &str,
    secs: u64,
) -> Result<tk::TkDuration, String> {
    if secs == 0 {
        Err(format!("{name} must be a positive number of seconds"))
    } else {
        Ok(tk::TkDuration::from_secs(secs))
    }
}

fn positive_rate(
    name: &str,
    rate: u32,
) -> Result<u32, String> {
    if rate == 0 {
        Err(format!("{name} must be greater than zero"))
    } else {
        Ok(rate)
    }
}

/// The host's ONE channel, reachable from an eval.
static CHANNEL: OnceLock<Arc<ChannelHandle>> = OnceLock::new();

pub(crate) fn channel_handle() -> Arc<ChannelHandle> {
    CHANNEL
        .get_or_init(|| Arc::new(ChannelHandle::new(config().channel.spam)))
        .clone()
}

fn claimed_of(inner: &ChannelInner) -> bool {
    inner
        .claimed
        .as_ref()
        .map(|c| c.load(Ordering::SeqCst))
        .unwrap_or(false)
}

fn push_locked(
    inner: &mut ChannelInner,
    line: String,
) -> Result<(), ChannelSendError> {
    match &mut inner.packets {
        Some(PacketSink::Hub(tx)) => tx.send(line).map_err(|_| ChannelSendError::HubGone),
        Some(PacketSink::Sync { lines, dropped }) => {
            if lines.len() >= SYNC_QUEUE_CAP {
                lines.pop_front();
                *dropped += 1;
            }
            lines.push_back(line);
            Ok(())
        }
        None => Err(ChannelSendError::HubGone),
    }
}

/// The teardown itself, with the lock already held.
fn close_locked(
    inner: &mut ChannelInner,
    code: u16,
    reason: &str,
    done: Option<tk::oneshot::Sender<()>>,
) {
    if let Some(tx) = inner.close.take() {
        let _ = tx.send((code, reason.to_string(), done));
    }
    inner.phase = ChannelPhase::Closed;
    inner.mode = None;
    inner.url = None;
    inner.packets = None;
    inner.claimed = None;
    inner.shutdown = None;
    inner.verify_cancel = None;
}

/// A channel message's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MsgId(datum::Nonce);

impl Display for MsgId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Mint a packet's id over the packet's own fields.
pub(crate) fn mint_msg_id(
    nonce_gen: &datum::NonceGenerator,
    from: &str,
    model: &str,
    event_nuon: &str,
    attached_nuon: Option<&str>,
) -> MsgId {
    MsgId(nonce_gen.generate_with(&(from, model, event_nuon, attached_nuon)))
}

/// Render a value as compact NUON - what the wire and the hash both see.
pub(crate) fn render_nuon(value: &nu::Value) -> Result<String, String> {
    nu::to_nuon(&nu::EngineState::new(), value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

/// Render one packet as the single NUON line the wire carries.
pub(crate) fn render_packet(
    id: MsgId,
    from: &str,
    model: &str,
    event: &nu::Value,
    attached: Option<&str>,
) -> Result<String, String> {
    let span = nu::Span::unknown();
    let mut record = nu::Record::new();
    record.insert("id", nu::Value::string(id.to_string(), span));
    record.insert("from", nu::Value::string(from.to_string(), span));
    record.insert("model", nu::Value::string(model.to_string(), span));
    record.insert("event", event.clone());
    if let Some(name) = attached {
        record.insert("attached", nu::Value::string(name.to_string(), span));
    }
    let rendered = render_nuon(&nu::Value::record(record, span))?;
    Ok(escape_line(&rendered))
}

/// Re-escape raw newlines so one packet is always one line.
pub(crate) fn escape_line(rendered: &str) -> String {
    rendered.replace('\n', "\\n").replace('\r', "\\r")
}

/// The client's task-notification cap on a rendered packet line, in characters.
pub(crate) const NOTIFICATION_CAP: usize = 500;

/// What the packet's wrapper takes of the cap around its event.
const NOTIFICATION_WRAPPER_RESERVE: usize = 120;

/// The event key marking a spilled event, its value the full event's path.
pub(crate) const EVENT_SPILL_KEY: &str = "spilled_event_path";
const EVENT_BYTES_KEY: &str = "event_bytes";

/// The dest a remote event spill transfers under, in its delivery's directory.
pub(crate) const EVENT_SPILL_DEST: &str = ".mcp_event.nuon";

/// Would this rendered event NUON overflow the notification cap once wrapped?
pub(crate) fn event_overflows(event_nuon: &str) -> bool {
    event_nuon.chars().count() > NOTIFICATION_CAP - NOTIFICATION_WRAPPER_RESERVE
}

/// The pointer event replacing an oversized one: the spill's path and its size.
pub(crate) fn event_spill_pointer(
    path: &str,
    event_bytes: usize,
) -> nu::Value {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    r.insert(EVENT_SPILL_KEY, nu::Value::string(path.to_string(), span));
    r.insert(EVENT_BYTES_KEY, nu::Value::int(event_bytes as i64, span));
    nu::Value::record(r, span)
}
