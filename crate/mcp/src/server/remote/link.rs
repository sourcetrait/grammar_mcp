#![allow(dead_code)]
//! Remote links: two known-public-key mTLS connections to one peer.
use crate::*;

/// The client handshake's server name, which the known-public-key match ignores.
const PLACEHOLDER_SERVER_NAME: &str = "grammar.invalid";

/// How long a closing side waits for the reciprocal `Close` before giving up.
const CLOSE_FRAME_GRACE: tk::TkDuration = tk::TkDuration::from_secs(2);

/// One file chunk's uncompressed payload ceiling; zstd + the frame ride on top.
const CHUNK_BYTES: usize = 256 * 1024;


/// One open remote link plus the peer address, for the registry and listing.
pub(crate) struct RemoteLinkEntry {
    pub handle: RemoteLinkHandle,
    pub addr: std::net::SocketAddr,
}

/// Every open link, keyed by alias, behind a std Mutex an eval thread can take.
static REMOTE_LINKS: LazyLock<Arc<std::sync::Mutex<HashMap<String, RemoteLinkEntry>>>> =
    LazyLock::new(|| Arc::new(std::sync::Mutex::new(HashMap::new())));

/// The shared registry; the Channel's face holds a clone, grimm sends reach it.
pub(crate) fn remote_links() -> Arc<std::sync::Mutex<HashMap<String, RemoteLinkEntry>>> {
    REMOTE_LINKS.clone()
}

/// A listener bound and awaiting its peer: its address and its accept's cancel.
pub(crate) struct BoundListener {
    pub addr: std::net::SocketAddr,
    seq: u64,
    cancel: tku::CancellationToken,
    released: Option<tk::oneshot::Receiver<()>>,
}

/// Bound-but-unpaired listeners, keyed by alias.
static BOUND_LISTENERS: LazyLock<Arc<std::sync::Mutex<HashMap<String, BoundListener>>>> =
    LazyLock::new(|| Arc::new(std::sync::Mutex::new(HashMap::new())));

/// Each bound listener's identity, so a task leaves only its own entry.
static LISTENER_SEQ: AtomicU64 = AtomicU64::new(0);

/// The shared bound-listener registry; remote_channels reads it for `listening`.
pub(crate) fn bound_listeners() -> Arc<std::sync::Mutex<HashMap<String, BoundListener>>> {
    BOUND_LISTENERS.clone()
}

/// Cancel a bound listener's accept; false when none is bound under `alias`.
pub(crate) async fn close_listener(alias: &str) -> bool {
    let entry = bound_listeners()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(alias);
    let Some(mut entry) = entry else {
        return false;
    };
    entry.cancel.cancel();
    if let Some(released) = entry.released.take() {
        let _ = tk::timeout(CLOSE_FRAME_GRACE, released).await;
    }
    true
}

/// Find the open link to `mcp_nom` and enqueue a send on it.
pub(crate) fn find_link_send(
    mcp_nom: &str,
    id: String,
    model: String,
    event_nuon: String,
    payloads: Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    let links = remote_links();
    let guard = links.lock().unwrap_or_else(|e| e.into_inner());
    let entry = guard
        .values()
        .find(|e| e.handle.remote_mcp_nom == mcp_nom)
        .ok_or_else(|| format!("no open remote link to mcp_nom `{mcp_nom}`"))?;
    entry.handle.enqueue_send(id, model, event_nuon, payloads)
}

/// Whether a dest is a relative path of plain components, never an escape.
pub(crate) fn safe_dest(dest: &str) -> bool {
    let path = std::path::Path::new(dest);
    !dest.is_empty()
        && path.is_relative()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// A delivery's file beneath `inbox`, none where a peer-named part isn't plain.
pub(crate) fn delivery_path(
    inbox: &std::path::Path,
    peer_nom: &str,
    id: &str,
    dest: &str,
) -> Option<PathBuf> {
    (valid_peer_id(peer_nom) && valid_peer_id(id) && safe_dest(dest))
        .then(|| inbox.join(peer_nom).join(id).join(dest))
}

/// How long a connector's dial may take before the open fails.
const CONNECT_TIMEOUT: tk::TkDuration = tk::TkDuration::from_secs(20);

/// Connect to `alias`'s peer at `addr`, then watch for its teardown behind.
pub(crate) async fn connect_remote(
    self_mcp_nom: &datum::NomPair,
    alias: String,
    keys: LinkKeys,
    addr: SocketAddr,
) -> Result<(), String> {
    let (handle, msg_join, file_join) =
        match tk::timeout(CONNECT_TIMEOUT, connect_link(self_mcp_nom, &keys, addr)).await {
            Ok(Ok(drivers)) => drivers,
            Ok(Err(e)) => return Err(format!("connect {addr}: {e}")),
            Err(_) => {
                return Err(format!(
                    "connect {addr} timed out after {}s",
                    CONNECT_TIMEOUT.as_secs(),
                ));
            }
        };
    let peer_nom = handle.remote_mcp_nom.clone();
    remote_links()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(alias.clone(), RemoteLinkEntry { handle, addr });
    tk::spawn(async move {
        let _ = msg_join.await;
        let _ = file_join.await;
        deregister_if_ours(&alias, &peer_nom);
        emit_lifecycle(MODEL_DISCONNECTED, &alias, &peer_nom);
    });
    Ok(())
}

/// Listen for `alias`'s peer on `bind`, answering the address bound.
pub(crate) async fn listen_remote(
    self_mcp_nom: &datum::NomPair,
    alias: String,
    keys: LinkKeys,
    bind: SocketAddr,
) -> Result<SocketAddr, String> {
    let (listener, acceptor) = bind_listener(&keys, bind)
        .await
        .map_err(|e| format!("bind {bind}: {e}"))?;
    let bound = listener
        .local_addr()
        .map_err(|e| format!("bind {bind}: {e}"))?;
    let seq = LISTENER_SEQ.fetch_add(1, Ordering::Relaxed);
    let cancel = tku::CancellationToken::new();
    let (released_tx, released_rx) = tk::oneshot::channel::<()>();
    bound_listeners().lock().unwrap_or_else(|e| e.into_inner()).insert(
        alias.clone(),
        BoundListener {
            addr: bound,
            seq,
            cancel: cancel.clone(),
            released: Some(released_rx),
        },
    );
    let self_mcp_nom = self_mcp_nom.clone();
    tk::spawn(async move {
        let accepted = tokio::select! {
            biased;
            _ = cancel.cancelled() => None,
            accepted = accept_and_pair(&self_mcp_nom, listener, acceptor) => Some(accepted),
        };
        match accepted {
            None => {
                let _ = released_tx.send(());
                emit_closed(&alias);
            }
            Some(Ok((handle, msg_join, file_join))) => {
                let peer_nom = handle.remote_mcp_nom.clone();
                let link = RemoteLinkEntry { handle, addr: bound };
                if let Err(link) = promote_listener(&alias, seq, link) {
                    link.handle.cancel();
                    let _ = released_tx.send(());
                    let _ = msg_join.await;
                    let _ = file_join.await;
                    emit_closed(&alias);
                    return;
                }
                drop(released_tx);
                emit_lifecycle(MODEL_CONNECTED, &alias, &peer_nom);
                let _ = msg_join.await;
                let _ = file_join.await;
                deregister_if_ours(&alias, &peer_nom);
                emit_lifecycle(MODEL_DISCONNECTED, &alias, &peer_nom);
            }
            Some(Err(e)) => {
                unlisten(&alias, seq);
                drop(released_tx);
                emit_open_failed(&alias, "listen", &e.to_string());
            }
        }
    });
    Ok(bound)
}

/// Move a paired listener's link into the registry, unless it was closed first.
fn promote_listener(
    alias: &str,
    seq: u64,
    link: RemoteLinkEntry,
) -> Result<(), RemoteLinkEntry> {
    let listeners = bound_listeners();
    let mut bound = listeners.lock().unwrap_or_else(|e| e.into_inner());
    if bound.get(alias).is_none_or(|entry| entry.seq != seq) {
        return Err(link);
    }
    remote_links()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(alias.to_string(), link);
    bound.remove(alias);
    Ok(())
}

/// Drop the bound-listener entry for `alias` if it is still this task's.
fn unlisten(
    alias: &str,
    seq: u64,
) {
    let listeners = bound_listeners();
    let mut bound = listeners.lock().unwrap_or_else(|e| e.into_inner());
    if bound.get(alias).is_some_and(|entry| entry.seq == seq) {
        bound.remove(alias);
    }
}

/// Drop `alias`'s link only while it is still the one this task established.
fn deregister_if_ours(
    alias: &str,
    peer_nom: &str,
) {
    let links_arc = remote_links();
    let mut links = links_arc.lock().unwrap_or_else(|e| e.into_inner());
    if links
        .get(alias)
        .is_some_and(|e| e.handle.remote_mcp_nom == peer_nom)
    {
        links.remove(alias);
    }
}

/// Emit a host-origin lifecycle packet {remote, mcp_nom} onto the local Channel.
fn emit_lifecycle(
    model: &str,
    remote: &str,
    peer_nom: &str,
) {
    let span = nu::Span::unknown();
    let mut event = nu::Record::new();
    event.insert("remote", nu::Value::string(remote.to_string(), span));
    event.insert("mcp_nom", nu::Value::string(peer_nom.to_string(), span));
    push_report(model, nu::Value::record(event, span));
}

/// Emit remote/channel/Disconnected {remote}: a listener closed before it paired.
fn emit_closed(alias: &str) {
    let span = nu::Span::unknown();
    let mut event = nu::Record::new();
    event.insert("remote", nu::Value::string(alias.to_string(), span));
    push_report(MODEL_DISCONNECTED, nu::Value::record(event, span));
}

/// Emit remote/channel/Disconnected {remote, error} when a link never established.
fn emit_open_failed(
    alias: &str,
    kind: &str,
    message: &str,
) {
    let span = nu::Span::unknown();
    let mut err = nu::Record::new();
    err.insert("kind", nu::Value::string(kind.to_string(), span));
    err.insert("message", nu::Value::string(message.to_string(), span));
    let mut event = nu::Record::new();
    event.insert("remote", nu::Value::string(alias.to_string(), span));
    event.insert("error", nu::Value::record(err, span));
    push_report(MODEL_DISCONNECTED, nu::Value::record(event, span));
}

/// The certificates a link presents and matches: this host's pair, the peer's.
pub(crate) struct LinkKeys {
    /// This host's own certificate, presented in either role.
    pub self_cert: PathBuf,
    /// The private key paired with `self_cert`.
    pub self_key: PathBuf,
    /// The peer's known certificate; the presented leaf must match it byte for byte.
    pub peer_cert: PathBuf,
}

/// A running link: the peer's nom, its outbound queues, and their shared cancel.
pub(crate) struct RemoteLinkHandle {
    pub remote_mcp_nom: String,
    cancel: tku::CancellationToken,
    /// Deliver to the message-connection driver.
    msg_out_tx: tk::UnboundedSender<MsgFrame>,
    /// FileChunk to the file-connection driver.
    file_out_tx: tk::UnboundedSender<FileFrame>,
    /// Aggregate write-outcome tracking behind the Sent/Unsent report.
    send: SendTracker,
}

impl RemoteLinkHandle {
    /// Cancel both drivers; the open task then reports the link's end.
    pub(crate) fn cancel(&self) {
        if !self.cancel.is_cancelled() {
            self.cancel.cancel();
        }
    }

    /// Register a send, then queue its frames; a closed queue fails it synchronously.
    pub(crate) fn enqueue_send(
        &self,
        id: String,
        model: String,
        event_nuon: String,
        payloads: Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        let dests: Vec<String> = payloads.iter().map(|(dest, _)| dest.clone()).collect();
        self.send.register(&id, &dests);
        match push_frames(
            &self.msg_out_tx,
            &self.file_out_tx,
            &id,
            model,
            event_nuon,
            payloads,
            dests,
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.send.forget(&id);
                Err(e)
            }
        }
    }
}

/// Queue one send's chunks, then its Deliver, each checked as its peer reads it.
pub(crate) fn push_frames(
    msg_out_tx: &tk::UnboundedSender<MsgFrame>,
    file_out_tx: &tk::UnboundedSender<FileFrame>,
    id: &str,
    model: String,
    event_nuon: String,
    payloads: Vec<(String, Vec<u8>)>,
    dests: Vec<String>,
) -> Result<(), String> {
    let deliver = MsgFrame::Deliver {
        id: id.to_string(),
        model,
        event_nuon,
        files: dests,
    };
    let deliver = match checked(InitiatorToAcceptor::Msg(deliver))? {
        InitiatorToAcceptor::Msg(deliver) => deliver,
        _ => unreachable!("checked as a message frame"),
    };
    let mut chunks = Vec::new();
    for (dest, bytes) in payloads {
        if bytes.is_empty() {
            chunks.push(FileFrame::Chunk {
                id: id.to_string(),
                dest,
                seq: 0,
                bytes: Vec::new(),
                last: true,
            });
            continue;
        }
        let total = bytes.len().div_ceil(CHUNK_BYTES);
        for (i, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
            chunks.push(FileFrame::Chunk {
                id: id.to_string(),
                dest: dest.clone(),
                seq: i as u32,
                bytes: chunk.to_vec(),
                last: i + 1 == total,
            });
        }
    }
    let chunks = chunks
        .into_iter()
        .map(|chunk| match checked(InitiatorToAcceptor::File(chunk))? {
            InitiatorToAcceptor::File(chunk) => Ok(chunk),
            _ => unreachable!("checked as a file frame"),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let closing = || "remote link is closing".to_string();
    for chunk in chunks {
        file_out_tx.send(chunk).map_err(|_| closing())?;
    }
    msg_out_tx.send(deliver).map_err(|_| closing())
}

/// `frame` back once its peer would take it.
fn checked(frame: InitiatorToAcceptor) -> Result<InitiatorToAcceptor, String> {
    check_outbound(&frame)?;
    Ok(frame)
}

/// A spawned link: its handle and both drivers' join handles.
type LinkDrivers = (RemoteLinkHandle, tk::JoinHandle<()>, tk::JoinHandle<()>);

/// Connect as the initiator: both mTLS connections, handshaked, then driven.
async fn connect_link(
    self_mcp_nom: &datum::NomPair,
    keys: &LinkKeys,
    addr: SocketAddr,
) -> io::Result<LinkDrivers> {
    let connector = tls::TlsConnector::from(client_config(keys)?);
    let (remote_msg, msg_read, msg_write) =
        connect_conn(&connector, addr, self_mcp_nom, RemoteStream::Message).await?;
    let (remote_file, file_read, file_write) =
        connect_conn(&connector, addr, self_mcp_nom, RemoteStream::File).await?;
    pair_check(&remote_msg, &remote_file)?;
    Ok(spawn_initiator_drivers(
        remote_msg,
        (msg_read, msg_write),
        (file_read, file_write),
    ))
}

/// Bind the listener and build its TLS acceptor, the listen's synchronous half.
async fn bind_listener(
    keys: &LinkKeys,
    bind: SocketAddr,
) -> io::Result<(tk::TcpListener, tls::TlsAcceptor)> {
    let tls_config = server_config(&keys.self_cert, &keys.self_key, &keys.peer_cert)?;
    let acceptor = tls::TlsAcceptor::from(tls_config);
    let listener = tk::TcpListener::bind(bind).await?;
    Ok((listener, acceptor))
}

/// Accept one peer's two connections on a bound listener and drive them.
async fn accept_and_pair(
    self_mcp_nom: &datum::NomPair,
    listener: tk::TcpListener,
    acceptor: tls::TlsAcceptor,
) -> io::Result<LinkDrivers> {
    let mut message: Option<(String, AcceptFramedRead, AcceptFramedWrite)> = None;
    let mut file: Option<(String, AcceptFramedRead, AcceptFramedWrite)> = None;
    while message.is_none() || file.is_none() {
        let (tcp, _) = listener.accept().await?;
        let (remote, stream, framed_read, framed_write) =
            accept_conn(&acceptor, self_mcp_nom, tcp).await?;
        match stream {
            RemoteStream::Message => message = Some((remote, framed_read, framed_write)),
            RemoteStream::File => file = Some((remote, framed_read, framed_write)),
        }
    }
    let (remote_msg, msg_read, msg_write) = message.expect("message half present");
    let (remote_file, file_read, file_write) = file.expect("file half present");
    pair_check(&remote_msg, &remote_file)?;
    Ok(spawn_acceptor_drivers(
        remote_msg,
        (msg_read, msg_write),
        (file_read, file_write),
    ))
}

/// Turn two handshaked initiator connections into a running link + its joins.
fn spawn_initiator_drivers(
    remote_mcp_nom: String,
    message: (InitFramedRead, InitFramedWrite),
    file: (InitFramedRead, InitFramedWrite),
) -> LinkDrivers {
    let cancel = tku::CancellationToken::new();
    let (msg_out_tx, msg_out_rx) = tk::unbounded_channel::<MsgFrame>();
    let (file_out_tx, file_out_rx) = tk::unbounded_channel::<FileFrame>();
    let recv = RecvContext::new(&remote_mcp_nom);
    let send = SendTracker::new(&remote_mcp_nom);
    let message_join = tk::spawn(run_initiator_message(
        cancel.clone(),
        message.0,
        message.1,
        msg_out_rx,
        recv.clone(),
        send.clone(),
    ));
    let file_join = tk::spawn(run_initiator_file(
        cancel.clone(),
        file.0,
        file.1,
        file_out_rx,
        recv,
        send.clone(),
    ));
    (
        RemoteLinkHandle {
            remote_mcp_nom,
            cancel,
            msg_out_tx,
            file_out_tx,
            send,
        },
        message_join,
        file_join,
    )
}

/// Turn two handshaked acceptor connections into a running link + its joins.
fn spawn_acceptor_drivers(
    remote_mcp_nom: String,
    message: (AcceptFramedRead, AcceptFramedWrite),
    file: (AcceptFramedRead, AcceptFramedWrite),
) -> LinkDrivers {
    let cancel = tku::CancellationToken::new();
    let (msg_out_tx, msg_out_rx) = tk::unbounded_channel::<MsgFrame>();
    let (file_out_tx, file_out_rx) = tk::unbounded_channel::<FileFrame>();
    let recv = RecvContext::new(&remote_mcp_nom);
    let send = SendTracker::new(&remote_mcp_nom);
    let message_join = tk::spawn(run_acceptor_message(
        cancel.clone(),
        message.0,
        message.1,
        msg_out_rx,
        recv.clone(),
        send.clone(),
    ));
    let file_join = tk::spawn(run_acceptor_file(
        cancel.clone(),
        file.0,
        file.1,
        file_out_rx,
        recv,
        send.clone(),
    ));
    (
        RemoteLinkHandle {
            remote_mcp_nom,
            cancel,
            msg_out_tx,
            file_out_tx,
            send,
        },
        message_join,
        file_join,
    )
}

/// Both connections of one link must report the same peer McpNom.
fn pair_check(
    message: &str,
    file: &str,
) -> io::Result<()> {
    if message == file {
        Ok(())
    } else {
        Err(io_other(format!(
            "link peers disagree: message conn `{message}`, file conn `{file}`"
        )))
    }
}

/// One inbound delivery's state, coordinating its message + file connections.
#[derive(Default)]
struct DeliverSlot {
    /// Set when the Deliver frame arrives.
    deliver: Option<DeliverInfo>,
    /// The manifest dests, set with the Deliver; None until it arrives.
    expected: Option<HashSet<String>>,
    /// Dests whose last chunk has landed.
    done: HashSet<String>,
}

/// The message half of a Deliver: what the relayed packet carries.
struct DeliverInfo {
    model: String,
    event_nuon: String,
}

/// What both drivers of a link share to land + relay an inbound delivery.
#[derive(Clone)]
struct RecvContext {
    /// Per-id slots, coordinating the message + file connections.
    slots: Arc<std::sync::Mutex<HashMap<String, DeliverSlot>>>,
    /// The peer's McpNom - the `from = remote/channel/<nom>` a relayed packet gets.
    peer_nom: String,
}

impl RecvContext {
    fn new(peer_nom: &str) -> Self {
        Self {
            slots: Arc::new(std::sync::Mutex::new(HashMap::new())),
            peer_nom: peer_nom.to_string(),
        }
    }
}

/// Each outbound send's outstanding writes, reported Sent or Unsent.
#[derive(Clone)]
struct SendTracker {
    sends: Arc<std::sync::Mutex<HashMap<String, PendingSend>>>,
    /// The peer's McpNom, stamped into the Sent/Unsent report event.
    peer_nom: String,
}

/// One send's unwritten file dests and whether its Deliver is still unwritten.
struct PendingSend {
    pending_dests: HashSet<String>,
    message_pending: bool,
}

impl PendingSend {
    fn complete(&self) -> bool {
        self.pending_dests.is_empty() && !self.message_pending
    }
}

impl SendTracker {
    fn new(peer_nom: &str) -> Self {
        Self {
            sends: Arc::new(std::sync::Mutex::new(HashMap::new())),
            peer_nom: peer_nom.to_string(),
        }
    }

    /// Register a send before its frames are queued.
    fn register(
        &self,
        id: &str,
        dests: &[String],
    ) {
        let mut map = self.sends.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(
            id.to_string(),
            PendingSend {
                pending_dests: dests.iter().cloned().collect(),
                message_pending: true,
            },
        );
    }

    /// Drop a registered send without a report.
    fn forget(
        &self,
        id: &str,
    ) {
        self.sends.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
    }

    /// One dest's last chunk wrote OK; fire Sent if it completes the send.
    fn dest_written(
        &self,
        id: &str,
        dest: &str,
    ) {
        let done = {
            let mut map = self.sends.lock().unwrap_or_else(|e| e.into_inner());
            let complete = match map.get_mut(id) {
                Some(send) => {
                    send.pending_dests.remove(dest);
                    send.complete()
                }
                None => false,
            };
            if complete {
                map.remove(id);
            }
            complete
        };
        if done {
            report_sent(&self.peer_nom, id);
        }
    }

    /// The Deliver wrote OK; fire Sent if it completes the send.
    fn message_written(
        &self,
        id: &str,
    ) {
        let done = {
            let mut map = self.sends.lock().unwrap_or_else(|e| e.into_inner());
            let complete = match map.get_mut(id) {
                Some(send) => {
                    send.message_pending = false;
                    send.complete()
                }
                None => false,
            };
            if complete {
                map.remove(id);
            }
            complete
        };
        if done {
            report_sent(&self.peer_nom, id);
        }
    }

    /// A write for this send failed; fire Unsent once (idempotent per id).
    fn failed(
        &self,
        id: &str,
        kind: &str,
        message: &str,
    ) {
        let present = self
            .sends
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id)
            .is_some();
        if present {
            report_unsent(&self.peer_nom, id, kind, message);
        }
    }

    /// Report every still-pending send Unsent, the link down; idempotent.
    fn flush_unsent(
        &self,
        kind: &str,
        message: &str,
    ) {
        let drained: Vec<String> = {
            let mut map = self.sends.lock().unwrap_or_else(|e| e.into_inner());
            map.drain().map(|(id, _)| id).collect()
        };
        for id in drained {
            report_unsent(&self.peer_nom, &id, kind, message);
        }
    }
}

/// Record the Deliver half of a delivery, then try to finalize.
fn record_deliver(
    ctx: &RecvContext,
    id: &str,
    info: DeliverInfo,
    files: Vec<String>,
) {
    {
        let mut map = ctx.slots.lock().unwrap_or_else(|e| e.into_inner());
        let slot = map.entry(id.to_string()).or_default();
        slot.deliver = Some(info);
        slot.expected = Some(files.into_iter().collect());
    }
    try_finalize(ctx, id);
}

/// Record that one dest's last chunk landed, then try to finalize.
fn record_file_done(
    ctx: &RecvContext,
    id: &str,
    dest: &str,
) {
    {
        let mut map = ctx.slots.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(id.to_string())
            .or_default()
            .done
            .insert(dest.to_string());
    }
    try_finalize(ctx, id);
}

/// Relay a delivery once its Deliver and every file have landed.
fn try_finalize(
    ctx: &RecvContext,
    id: &str,
) {
    let taken = {
        let mut map = ctx.slots.lock().unwrap_or_else(|e| e.into_inner());
        let ready = map.get(id).is_some_and(|slot| {
            slot.deliver.is_some()
                && slot
                    .expected
                    .as_ref()
                    .is_some_and(|want| want.is_subset(&slot.done))
        });
        if ready { map.remove(id) } else { None }
    };
    let Some(slot) = taken else { return };
    let info = slot.deliver.expect("ready implies a deliver");
    let has_files = slot.expected.is_some_and(|want| !want.is_empty());
    if let Err((kind, message)) = relay_to_channel(ctx, id, &info, has_files) {
        eprintln!("grammar: remote delivery `{id}` not relayed onto the Channel: {kind}: {message}");
    }
}

/// The running Channel's inbox, read at each chunk since each open makes its own.
async fn channel_inbox() -> io::Result<PathBuf> {
    let host = host_channel().ok_or_else(|| io::Error::other("the Channel has no inbox"))?;
    let status = host.channel.request(StatusRequest).await.map_err(io::Error::other)?;
    status
        .inbox
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("the Channel has no inbox"))
}

/// Append one chunk to `<inbox>/<peer_nom>/<id>/<dest>`; true iff `last`.
fn land_file_chunk(
    ctx: &RecvContext,
    inbox: &Path,
    id: &str,
    dest: &str,
    seq: u32,
    bytes: &[u8],
    last: bool,
) -> io::Result<bool> {
    let path = delivery_path(inbox, &ctx.peer_nom, id, dest)
        .ok_or_else(|| io::Error::other("a delivery's path that isn't beneath the inbox"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = if seq == 0 {
        fs::File::create(&path)?
    } else {
        fs::OpenOptions::new().append(true).open(&path)?
    };
    file.write_all(bytes)?;
    Ok(last)
}

/// Relay an inbound delivery onto the local Channel through its verified gate.
fn relay_to_channel(
    ctx: &RecvContext,
    id: &str,
    info: &DeliverInfo,
    has_files: bool,
) -> Result<(), (String, String)> {
    if let Some(prefix) = reserved_prefix(&info.model) {
        return Err((
            "reserved_model".to_string(),
            format!("model `{}` lies under the host's `{prefix}` reservation", info.model),
        ));
    }
    let event = nu::from_nuon(&info.event_nuon, None)
        .map_err(|e| ("event_unparseable".to_string(), e.to_string()))?;
    let event = rewrite_spill_pointer(event, &ctx.peer_nom, id);
    let event = NuVal::try_from_nu(event).map_err(|e| ("render".to_string(), e.to_string()))?;
    let host = host_channel().ok_or_else(|| {
        ("channel".to_string(), ChannelSendError::NotOpen.message().to_string())
    })?;
    host.post(Post {
        emit: EmitRequest {
            from: format!("{REMOTE_RESERVED_PREFIX}channel/{}", ctx.peer_nom),
            model: info.model.clone(),
            event,
            attached: has_files.then(|| format!("{}/{}", ctx.peer_nom, id)),
        },
        delivery: Some(id.to_string()),
    });
    Ok(())
}

/// A spill pointer's dest made inbox-relative, under this delivery's directory.
fn rewrite_spill_pointer(
    event: nu::Value,
    peer_nom: &str,
    id: &str,
) -> nu::Value {
    let span = nu::Span::unknown();
    let Ok(rec) = event.as_record() else {
        return event;
    };
    let Some(dest) = rec.get(EVENT_SPILL_KEY).and_then(|v| v.as_str().ok()) else {
        return event;
    };
    if dest.contains('/') {
        return event;
    }
    let full = format!("{peer_nom}/{id}/{dest}");
    let mut new = rec.clone();
    new.insert(EVENT_SPILL_KEY, nu::Value::string(full, span));
    nu::Value::record(new, span)
}

/// Push a Sent report {id, mcp_nom} onto the LOCAL (sender's own) Channel.
fn report_sent(
    peer_nom: &str,
    id: &str,
) {
    let span = nu::Span::unknown();
    let mut event = nu::Record::new();
    event.insert("id", nu::Value::string(id.to_string(), span));
    event.insert("mcp_nom", nu::Value::string(peer_nom.to_string(), span));
    push_report(MODEL_SENT, nu::Value::record(event, span));
}

/// Push an Unsent report onto the sender's own Channel.
fn report_unsent(
    peer_nom: &str,
    id: &str,
    kind: &str,
    message: &str,
) {
    let span = nu::Span::unknown();
    let mut err = nu::Record::new();
    err.insert("kind", nu::Value::string(kind.to_string(), span));
    err.insert("message", nu::Value::string(message.to_string(), span));
    let mut event = nu::Record::new();
    event.insert("id", nu::Value::string(id.to_string(), span));
    event.insert("mcp_nom", nu::Value::string(peer_nom.to_string(), span));
    event.insert("error", nu::Value::record(err, span));
    push_report(MODEL_UNSENT, nu::Value::record(event, span));
}

/// Emit a host-origin packet onto the local Channel, best effort.
fn push_report(
    model: &str,
    event: nu::Value,
) {
    let (Some(host), Ok(event)) = (host_channel(), NuVal::try_from_nu(event)) else {
        return;
    };
    host.post(Post {
        emit: EmitRequest {
            from: FROM_MCP.to_string(),
            model: model.to_string(),
            event,
            attached: None,
        },
        delivery: None,
    });
}

/// Handle one inbound message-connection frame (a Deliver to relay).
fn handle_msg_frame(
    recv: &RecvContext,
    frame: MsgFrame,
) {
    let MsgFrame::Deliver {
        id,
        model,
        event_nuon,
        files,
    } = frame;
    if !valid_peer_id(&id) {
        eprintln!("grammar: remote delivery whose id isn't base62; dropped");
        return;
    }
    if let Some(dest) = files.iter().find(|dest| !safe_dest(dest)) {
        eprintln!("grammar: remote delivery `{id}` names unsafe dest `{dest}`; dropped");
        return;
    }
    record_deliver(recv, &id, DeliverInfo { model, event_nuon }, files);
}

/// Handle one inbound file-connection frame (receiver side).
async fn handle_file_frame(
    recv: &RecvContext,
    frame: FileFrame,
) {
    let FileFrame::Chunk {
        id,
        dest,
        seq,
        bytes,
        last,
    } = frame;
    if !valid_peer_id(&id) {
        eprintln!("grammar: remote file chunk whose delivery id isn't base62; dropped");
        return;
    }
    if !safe_dest(&dest) {
        eprintln!("grammar: remote file chunk for `{id}` has unsafe dest `{dest}`; dropped");
        return;
    }
    let landed = match channel_inbox().await {
        Ok(inbox) => land_file_chunk(recv, &inbox, &id, &dest, seq, &bytes, last),
        Err(e) => Err(e),
    };
    match landed {
        Ok(true) => record_file_done(recv, &id, &dest),
        Ok(false) => {}
        Err(e) => eprintln!("grammar: remote file chunk write failed for `{id}`/`{dest}`: {e}"),
    }
}

type InitReadHalf = tk::ReadHalf<tls::client::TlsStream<tk::TcpStream>>;
type InitWriteHalf = tk::WriteHalf<tls::client::TlsStream<tk::TcpStream>>;
type InitFramedRead = tku::FramedRead<InitReadHalf, BitcodeCodec<AcceptorToInitiator>>;
type InitFramedWrite = tku::FramedWrite<InitWriteHalf, BitcodeCodec<InitiatorToAcceptor>>;

/// Connect one mTLS connection and exchange the McpNom handshake.
async fn connect_conn(
    connector: &tls::TlsConnector,
    addr: std::net::SocketAddr,
    self_mcp_nom: &datum::NomPair,
    stream: RemoteStream,
) -> io::Result<(String, InitFramedRead, InitFramedWrite)> {
    let tcp = tk::TcpStream::connect(addr).await?;
    let domain = rv::ServerName::try_from(PLACEHOLDER_SERVER_NAME).map_err(io_other)?;
    let tls = connector.connect(domain, tcp).await?;
    let (read, write) = tk::split(tls);
    let mut framed_read = tku::FramedRead::new(read, BitcodeCodec::<AcceptorToInitiator>::new());
    let mut framed_write = tku::FramedWrite::new(write, BitcodeCodec::<InitiatorToAcceptor>::new());
    framed_write
        .send(InitiatorToAcceptor::Hello {
            mcp_nom: self_mcp_nom.as_str().to_string(),
            stream,
        })
        .await?;
    let remote = match framed_read.next().await {
        Some(Ok(AcceptorToInitiator::Hello { mcp_nom })) => mcp_nom,
        Some(Ok(_)) => return Err(io_other("peer sent a non-Hello during handshake")),
        Some(Err(e)) => return Err(e),
        None => return Err(io_other("peer closed during handshake")),
    };
    if !valid_peer_id(&remote) {
        return Err(io_other("peer named a nom that isn't base62"));
    }
    Ok((remote, framed_read, framed_write))
}

/// Drive the initiator's message connection, its Deliver frames both ways.
async fn run_initiator_message(
    cancel: tku::CancellationToken,
    mut framed_read: InitFramedRead,
    mut framed_write: InitFramedWrite,
    mut msg_out_rx: tk::UnboundedReceiver<MsgFrame>,
    recv: RecvContext,
    send: SendTracker,
) {
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                close_initiator(&mut framed_read, &mut framed_write).await;
                break;
            }
            out = msg_out_rx.recv() => match out {
                Some(frame) => {
                    let MsgFrame::Deliver { id, .. } = &frame;
                    let id = id.clone();
                    match framed_write.send(InitiatorToAcceptor::Msg(frame)).await {
                        Ok(()) => send.message_written(&id),
                        Err(_) => {
                            send.failed(&id, "write", "message connection write failed");
                            cancel.cancel();
                            break;
                        }
                    }
                }
                None => break,
            },
            incoming = framed_read.next() => match incoming {
                Some(Ok(AcceptorToInitiator::Msg(frame))) => handle_msg_frame(&recv, frame),
                Some(Ok(AcceptorToInitiator::Close)) => {
                    let _ = framed_write.send(InitiatorToAcceptor::Close).await;
                    break;
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
        }
    }
    let _ = framed_write.close().await;
    send.flush_unsent("link_down", "the remote link closed before the send completed");
}

/// Drive the initiator's FILE connection: FileChunk in (land) and out (send).
async fn run_initiator_file(
    cancel: tku::CancellationToken,
    mut framed_read: InitFramedRead,
    mut framed_write: InitFramedWrite,
    mut file_out_rx: tk::UnboundedReceiver<FileFrame>,
    recv: RecvContext,
    send: SendTracker,
) {
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                close_initiator(&mut framed_read, &mut framed_write).await;
                break;
            }
            out = file_out_rx.recv() => match out {
                Some(frame) => {
                    let FileFrame::Chunk { id, dest, last, .. } = &frame;
                    let (id, dest, last) = (id.clone(), dest.clone(), *last);
                    match framed_write.send(InitiatorToAcceptor::File(frame)).await {
                        Ok(()) => {
                            if last {
                                send.dest_written(&id, &dest);
                            }
                        }
                        Err(_) => {
                            send.failed(&id, "write", "file connection write failed");
                            cancel.cancel();
                            break;
                        }
                    }
                }
                None => break,
            },
            incoming = framed_read.next() => match incoming {
                Some(Ok(AcceptorToInitiator::File(frame))) => handle_file_frame(&recv, frame).await,
                Some(Ok(AcceptorToInitiator::Close)) => {
                    let _ = framed_write.send(InitiatorToAcceptor::Close).await;
                    break;
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
        }
    }
    let _ = framed_write.close().await;
    send.flush_unsent("link_down", "the remote link closed before the send completed");
}

/// Send `Close` and wait briefly for the peer's reciprocal `Close`.
async fn close_initiator(
    framed_read: &mut InitFramedRead,
    framed_write: &mut InitFramedWrite,
) {
    if framed_write.send(InitiatorToAcceptor::Close).await.is_err() {
        return;
    }
    let _ = tk::timeout(CLOSE_FRAME_GRACE, async {
        loop {
            match framed_read.next().await {
                Some(Ok(AcceptorToInitiator::Close)) => break,
                Some(Ok(_)) => continue,
                Some(Err(_)) => break,
                None => break,
            }
        }
    })
    .await;
}

type AcceptReadHalf = tk::ReadHalf<tls::server::TlsStream<tk::TcpStream>>;
type AcceptWriteHalf = tk::WriteHalf<tls::server::TlsStream<tk::TcpStream>>;
type AcceptFramedRead = tku::FramedRead<AcceptReadHalf, BitcodeCodec<InitiatorToAcceptor>>;
type AcceptFramedWrite = tku::FramedWrite<AcceptWriteHalf, BitcodeCodec<AcceptorToInitiator>>;

/// Accept one mTLS connection and answer the McpNom handshake.
async fn accept_conn(
    acceptor: &tls::TlsAcceptor,
    self_mcp_nom: &datum::NomPair,
    tcp: tk::TcpStream,
) -> io::Result<(String, RemoteStream, AcceptFramedRead, AcceptFramedWrite)> {
    let tls = acceptor.accept(tcp).await?;
    let (read, write) = tk::split(tls);
    let mut framed_read = tku::FramedRead::new(read, BitcodeCodec::<InitiatorToAcceptor>::new());
    let mut framed_write = tku::FramedWrite::new(write, BitcodeCodec::<AcceptorToInitiator>::new());
    let (remote, stream) = match framed_read.next().await {
        Some(Ok(InitiatorToAcceptor::Hello { mcp_nom, stream })) => (mcp_nom, stream),
        Some(Ok(_)) => return Err(io_other("peer sent a non-Hello during handshake")),
        Some(Err(e)) => return Err(e),
        None => return Err(io_other("peer closed during handshake")),
    };
    if !valid_peer_id(&remote) {
        return Err(io_other("peer named a nom that isn't base62"));
    }
    framed_write
        .send(AcceptorToInitiator::Hello {
            mcp_nom: self_mcp_nom.as_str().to_string(),
        })
        .await?;
    Ok((remote, stream, framed_read, framed_write))
}

/// Drive the acceptor's message connection, its Deliver frames both ways.
async fn run_acceptor_message(
    cancel: tku::CancellationToken,
    mut framed_read: AcceptFramedRead,
    mut framed_write: AcceptFramedWrite,
    mut msg_out_rx: tk::UnboundedReceiver<MsgFrame>,
    recv: RecvContext,
    send: SendTracker,
) {
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                close_acceptor(&mut framed_read, &mut framed_write).await;
                break;
            }
            out = msg_out_rx.recv() => match out {
                Some(frame) => {
                    let MsgFrame::Deliver { id, .. } = &frame;
                    let id = id.clone();
                    match framed_write.send(AcceptorToInitiator::Msg(frame)).await {
                        Ok(()) => send.message_written(&id),
                        Err(_) => {
                            send.failed(&id, "write", "message connection write failed");
                            cancel.cancel();
                            break;
                        }
                    }
                }
                None => break,
            },
            incoming = framed_read.next() => match incoming {
                Some(Ok(InitiatorToAcceptor::Msg(frame))) => handle_msg_frame(&recv, frame),
                Some(Ok(InitiatorToAcceptor::Close)) => {
                    let _ = framed_write.send(AcceptorToInitiator::Close).await;
                    break;
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
        }
    }
    let _ = framed_write.close().await;
    send.flush_unsent("link_down", "the remote link closed before the send completed");
}

/// Drive the acceptor's FILE connection: FileChunk in (land) and out (send).
async fn run_acceptor_file(
    cancel: tku::CancellationToken,
    mut framed_read: AcceptFramedRead,
    mut framed_write: AcceptFramedWrite,
    mut file_out_rx: tk::UnboundedReceiver<FileFrame>,
    recv: RecvContext,
    send: SendTracker,
) {
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                close_acceptor(&mut framed_read, &mut framed_write).await;
                break;
            }
            out = file_out_rx.recv() => match out {
                Some(frame) => {
                    let FileFrame::Chunk { id, dest, last, .. } = &frame;
                    let (id, dest, last) = (id.clone(), dest.clone(), *last);
                    match framed_write.send(AcceptorToInitiator::File(frame)).await {
                        Ok(()) => {
                            if last {
                                send.dest_written(&id, &dest);
                            }
                        }
                        Err(_) => {
                            send.failed(&id, "write", "file connection write failed");
                            cancel.cancel();
                            break;
                        }
                    }
                }
                None => break,
            },
            incoming = framed_read.next() => match incoming {
                Some(Ok(InitiatorToAcceptor::File(frame))) => handle_file_frame(&recv, frame).await,
                Some(Ok(InitiatorToAcceptor::Close)) => {
                    let _ = framed_write.send(AcceptorToInitiator::Close).await;
                    break;
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
        }
    }
    let _ = framed_write.close().await;
    send.flush_unsent("link_down", "the remote link closed before the send completed");
}

/// Send `Close` and wait briefly for the peer's reciprocal `Close`.
async fn close_acceptor(
    framed_read: &mut AcceptFramedRead,
    framed_write: &mut AcceptFramedWrite,
) {
    if framed_write.send(AcceptorToInitiator::Close).await.is_err() {
        return;
    }
    let _ = tk::timeout(CLOSE_FRAME_GRACE, async {
        loop {
            match framed_read.next().await {
                Some(Ok(InitiatorToAcceptor::Close)) => break,
                Some(Ok(_)) => continue,
                Some(Err(_)) => break,
                None => break,
            }
        }
    })
    .await;
}

/// Build the client config: present our public key, match the peer's known one.
fn client_config(opts: &LinkKeys) -> io::Result<Arc<tls::ClientConfig>> {
    let verifier = Arc::new(PublicKeyVerifier::new(load_public_key(&opts.peer_cert)?));
    let provider = Arc::new(rv::default_provider());
    let config = tls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io_other)?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(
            load_chain(&opts.self_cert)?,
            load_key(&opts.self_key)?,
        )
        .map_err(io_other)?;
    Ok(Arc::new(config))
}

/// Build the server config: present our public key, match the client's known one.
fn server_config(
    self_cert: &std::path::Path,
    self_key: &std::path::Path,
    peer_cert: &std::path::Path,
) -> io::Result<Arc<tls::ServerConfig>> {
    let verifier = Arc::new(PublicKeyVerifier::new(load_public_key(peer_cert)?));
    let provider = Arc::new(rv::default_provider());
    let config = tls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io_other)?
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            load_chain(self_cert)?,
            load_key(self_key)?,
        )
        .map_err(io_other)?;
    Ok(Arc::new(config))
}

fn load_public_key(path: &std::path::Path) -> io::Result<rv::CertificateDer<'static>> {
    rv::CertificateDer::from_pem_file(path)
        .map_err(|e| io_other(format!("load public key {}: {e}", path.display())))
}

fn load_chain(path: &std::path::Path) -> io::Result<Vec<rv::CertificateDer<'static>>> {
    Ok(vec![load_public_key(path)?])
}

fn load_key(path: &std::path::Path) -> io::Result<tls::PrivateKeyDer<'static>> {
    tls::PrivateKeyDer::from_pem_file(path)
        .map_err(|e| io_other(format!("load key {}: {e}", path.display())))
}

fn io_other<E: Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}
