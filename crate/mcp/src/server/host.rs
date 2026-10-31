//! The engine's start and its host: what an eval asks of mcp, on its thread.
use crate::*;

/// Start the engine on the config's id, namespace, and homes, mcp its host.
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
    engine::init(params, Box::new(McpHost)).map_err(|e| e.to_string())
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

/// The engine's host: the Channel, the remote links, the config, and the pins.
pub(crate) struct McpHost;

impl EngineHost for McpHost {
    fn channel_send(
        &self,
        send: ChannelSend<'_>,
    ) -> Result<String, String> {
        let ChannelSend {
            from,
            model,
            event,
            attached,
            stop,
        } = send;
        let Some(host) = host_channel() else {
            return Err(ChannelSendError::NotOpen.message().to_string());
        };
        let data = ChannelSendData {
            from: from.clone(),
            model,
            event: sendable(event)?,
            attached: attached.map(sendable).transpose()?,
        };
        match host.channel.request_blocking(SendRequest(data))? {
            SendVerdict::Sent(id) => Ok(id),
            SendVerdict::Refused(reason) => Err(reason),
            SendVerdict::Stop(spam) => {
                let action = stop();
                if spam.notify {
                    let stopped = ChannelSpamErrorEmergency {
                        origin: from,
                        hits: spam.hits,
                        window_secs: spam.window_secs,
                        rate: spam.rate,
                        action,
                    };
                    let _ = host.channel.notify_blocking(StoppedNotice(stopped));
                }
                Err(spam.reason)
            }
        }
    }

    fn remote_send(
        &self,
        send: RemoteSend,
    ) -> Result<String, String> {
        let RemoteSend {
            mcp_nom,
            model,
            event,
            attached,
        } = send;
        reject_reserved_model(&model)?;
        let payloads = match &attached {
            Some(value) => read_attachments(value)?,
            None => Vec::new(),
        };
        let event_nuon = render_nuon(&event)?;
        let Some(host) = host_channel() else {
            return Err(format!("no open remote link to mcp_nom `{mcp_nom}`"));
        };
        let send = RemoteSendRequest {
            mcp_nom,
            model,
            event_nuon,
            payloads,
        };
        host.peer.request_blocking(send)?
    }

    fn config_record(&self) -> nu::Value {
        config_record()
    }

    fn pin_config(
        &self,
        key: &str,
        pid: u32,
        value: f64,
    ) -> Result<(), String> {
        pin(key, pid, value)
    }
}

/// Refuse a model under the host's `mcp/` or `remote/` reservation.
fn reject_reserved_model(model: &str) -> Result<(), String> {
    if let Some(prefix) = reserved_prefix(model) {
        return Err(format!(
            "`{prefix}` is reserved for host-originated models; \
             choose a model path outside it",
        ));
    }
    Ok(())
}

/// A value as the data a send carries; one nuval can't carry fails as its render.
fn sendable(value: nu::Value) -> Result<NuVal, String> {
    NuVal::try_from_nu(value.clone()).map_err(|e| match render_nuon(&value) {
        Err(render) => render,
        Ok(_) => e.to_string(),
    })
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

/// The whole config as a nu record, mirroring the TOML's tables.
fn config_record() -> nu::Value {
    let span = nu::Span::unknown();
    let cfg = config();
    let spam = live_thresholds();
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
    nu::Value::record(root, span)
}

/// The spam thresholds in force: the running Channel's, else the config's start.
fn live_thresholds() -> SpamThresholdsData {
    host_channel()
        .and_then(|host| host.channel.request_blocking(ThresholdsRequest::default()).ok())
        .and_then(Result::ok)
        .unwrap_or_else(|| config().channel.spam.data())
}
