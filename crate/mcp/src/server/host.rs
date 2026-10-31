//! The engine's start, and the engine's host asks routed by mcp's controller.
use crate::*;

/// Start the engine on the config's id, namespace, and homes.
pub(crate) fn init_engine() -> Result<(), String> {
    let cfg = config();
    let scratch = make_scratch_dirs()?;
    let (_, mcp_nom) = process_identity();
    let params = EngineParams {
        id: cfg.id.clone(),
        namespace: cfg.namespace.clone(),
        work_dir: cfg.work_dir.clone(),
        tmp_dir: scratch.tmp,
        transient_dir: scratch.transient,
        mcp_nom: mcp_nom.as_str().to_string(),
        data_home: data_base_dir(),
        cache_home: cache_base_dir(),
    };
    engine::init(params).map_err(|e| e.to_string())
}

/// The substrate: the equip store, the default purview, the registry, builtins.
pub(crate) async fn ensure_substrate() -> io::Result<()> {
    let fresh = ensure_equip_store()?;
    if let Err(e) = ensure_default_purview() {
        eprintln!("grammar: default purview not materialized: {e:?}");
    }
    ensure_plugin_registry();
    if fresh {
        seed_builtin_purviews(&install_builtins());
    }
    Ok(())
}

/// mcp's controller: the engine's asks routed to their systems, and its policy.
#[derive(Clone)]
pub(crate) struct HostRoute {
    pub(crate) channel: Arc<ChannelHandle>,
    pub(crate) peer: Arc<PeerHandle>,
    /// The face's hub, through which the Channel's own emergencies reach the policy.
    hub: Arc<HubSlot>,
    /// The policy over the systems' emergencies.
    policy: EmergencyPolicy,
}

impl HostRoute {
    /// The route to a face's systems.
    pub(crate) fn of(face: &ChannelFace) -> Self {
        let mcp_nom = face.mcp_nom.as_str();
        let log = emergency_log_path(mcp_nom);
        Self {
            channel: face.channel.clone(),
            peer: face.peer.clone(),
            hub: face.hub.clone(),
            policy: EmergencyPolicy::new(face.channel.clone(), mcp_nom, log),
        }
    }

    /// Supervision's start: the Channel's own emergencies answered by the policy too.
    pub(crate) fn supervise(&self) {
        self.hub.install_policy(self.policy.clone());
    }

    /// An eval's ask, answered by the system it is for.
    pub(crate) async fn answer(
        &self,
        ask: HostAsk,
    ) -> HostAnswer {
        match ask {
            HostAsk::ChannelSend(data) => self.channel_send(data).await,
            HostAsk::Stopped(stopped) => {
                let _ = self.channel.notify(StoppedNotice(stopped)).await;
                HostAnswer::Noted
            }
            HostAsk::RemoteSend(data) => self.remote_send(data).await,
            HostAsk::Config => match config_record(&self.channel).await {
                Ok(record) => HostAnswer::Config(record),
                Err(reason) => HostAnswer::Refused(reason),
            },
            HostAsk::Pin(PinAsk { key, pid, value }) => match pin(&key, pid, value) {
                Ok(()) => HostAnswer::Pinned,
                Err(reason) => HostAnswer::Refused(reason),
            },
        }
    }

    /// A body's Channel send through the Channel's system, its verdict answered.
    async fn channel_send(
        &self,
        data: ChannelSendData,
    ) -> HostAnswer {
        match self.channel.request(SendRequest(data)).await {
            Ok(SendVerdict::Sent(id)) => HostAnswer::Sent(id),
            Ok(SendVerdict::Refused(reason)) | Err(reason) => HostAnswer::Refused(reason),
            Ok(SendVerdict::Stop(spam)) => HostAnswer::Stopped(HostStop {
                hits: spam.hits,
                window_secs: spam.window_secs,
                rate: spam.rate,
                notify: spam.notify,
                reason: spam.reason,
            }),
        }
    }

    /// A body's remote send, its files read and its event rendered, to the peer links.
    async fn remote_send(
        &self,
        data: RemoteSendData,
    ) -> HostAnswer {
        let RemoteSendData {
            mcp_nom,
            model,
            event,
            attached,
        } = data;
        let (event_nuon, payloads) = match tk::spawn_blocking(move || prepared(event, attached)).await {
            Ok(Ok(prepared)) => prepared,
            Ok(Err(reason)) => return HostAnswer::Refused(reason),
            Err(e) => return HostAnswer::Refused(format!("the send's files: {e}")),
        };
        let send = RemoteSendRequest {
            mcp_nom,
            model,
            event_nuon,
            payloads,
        };
        match self.peer.request(send).await {
            Ok(Ok(id)) => HostAnswer::Sent(id),
            Ok(Err(reason)) | Err(reason) => HostAnswer::Refused(reason),
        }
    }
}

impl subsys::Handler<McpSystem> for HostRoute {
    fn on_packet(&mut self, pkt: subsys::Packet<FromMcpSys>) -> impl Future<Output = Option<subsys::Packet<FromMcpSys>>> + 'static + Send {
        let policy = self.policy.clone();
        async move {
            if let FromMcpSys::Emergency(emergency) = pkt.take_msg() {
                policy.respond(emergency).await;
            }
            None
        }
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromMcpSys>) -> impl Future<Output = Option<ToMcpSys>> + 'static + Send {
        let route = self.clone();
        async move {
            match pkt.take_msg() {
                FromMcpSys::Host(ask) => Some(ToMcpSys::Host(route.answer(ask).await)),
                _ => None,
            }
        }
    }
}

/// A remote send's files read and its event rendered, the sender's own failures.
fn prepared(
    event: NuVal,
    attached: Option<NuVal>,
) -> Result<(String, Vec<Payload>), String> {
    let span = nu::Span::unknown();
    let payloads = match attached {
        Some(table) => read_attachments(&table.try_into_nu(span).map_err(|e| e.to_string())?)?,
        None => Vec::new(),
    };
    let event = event.try_into_nu(span).map_err(|e| e.to_string())?;
    Ok((render_nuon(&event)?, payloads))
}

/// Read each `{src, dest}` row into its payload, its dest checked first.
fn read_attachments(value: &nu::Value) -> Result<Vec<Payload>, String> {
    let rows = match value {
        nu::Value::List { vals, .. } => vals,
        _ => return Err("attached must be a table<src: path, dest: path>".to_string()),
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let rec = match row {
            nu::Value::Record { val, .. } => val,
            _ => return Err("each attached row must be a record with `src` and `dest`".to_string()),
        };
        let src = rec
            .get("src")
            .and_then(|v| v.as_str().ok())
            .ok_or_else(|| "each attached row needs a `src` path".to_string())?;
        let dest = rec
            .get("dest")
            .and_then(|v| v.as_str().ok())
            .ok_or_else(|| "each attached row needs a `dest` name".to_string())?;
        if !safe_dest(dest) {
            return Err(format!("dest `{dest}` must be a relative path with no `..`"));
        }
        let bytes = fs::read(src).map_err(|e| format!("read src `{src}`: {e}"))?;
        out.push(Payload {
            dest: dest.to_string(),
            bytes,
        });
    }
    Ok(out)
}

/// The whole config as a record mirroring the TOML's tables, with what is in force.
async fn config_record(handle: &ChannelHandle) -> Result<NuVal, String> {
    let span = nu::Span::unknown();
    let cfg = config();
    let spam = live_thresholds(handle).await;
    let supervisor = effective_supervisor(cfg.supervisor);

    let mut channel = nu::Record::new();
    channel.insert(
        "port",
        match cfg.channel.port {
            Some(port) => nu::Value::int(port as i64, span),
            None => nu::Value::nothing(span),
        },
    );
    channel.insert(
        "spam_warn_window_secs",
        nu::Value::int(spam.warn_window_secs as i64, span),
    );
    channel.insert("spam_warn_rate", nu::Value::int(spam.warn_rate as i64, span));
    channel.insert(
        "spam_error_window_secs",
        nu::Value::int(spam.error_window_secs as i64, span),
    );
    channel.insert(
        "spam_error_rate",
        nu::Value::int(spam.error_rate as i64, span),
    );

    let mut sup = nu::Record::new();
    sup.insert(
        "cpu_warn_fraction",
        nu::Value::float(supervisor.cpu_warn_fraction, span),
    );
    sup.insert(
        "ram_warn_fraction",
        nu::Value::float(supervisor.ram_warn_fraction, span),
    );
    sup.insert(
        "vram_warn_headroom_mib",
        nu::Value::int(supervisor.vram_warn_headroom_mib as i64, span),
    );
    sup.insert(
        "disk_warn_fraction",
        nu::Value::float(supervisor.disk_warn_fraction, span),
    );

    let identity = match &cfg.identity {
        Some(identity) => {
            let path = |file: &Path| nu::Value::string(file.to_string_lossy().into_owned(), span);
            let mut record = nu::Record::new();
            record.insert("cert", path(&identity.cert));
            record.insert("key", path(&identity.key));
            nu::Value::record(record, span)
        }
        None => nu::Value::nothing(span),
    };

    let mut root = nu::Record::new();
    root.insert("channel", nu::Value::record(channel, span));
    root.insert("supervisor", nu::Value::record(sup, span));
    root.insert("identity", identity);
    NuVal::try_from_nu(nu::Value::record(root, span)).map_err(|e| e.to_string())
}

/// The spam thresholds in force: the running Channel's, else the config's start.
async fn live_thresholds(handle: &ChannelHandle) -> SpamThresholdsData {
    match handle.request(ThresholdsRequest::default()).await {
        Ok(Ok(thresholds)) => thresholds,
        _ => config().channel.spam.data(),
    }
}
