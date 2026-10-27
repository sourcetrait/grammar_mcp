//! The engine's host: what an eval asks of mcp, answered on the eval's thread.
use crate::*;

/// RFC 6455 policy violation - the peer was emitted to before verifying.
const CLOSE_UNVERIFIED_EMIT: u16 = 1008;
const UNVERIFIED_EMIT_REASON: &str = "emit before verification";

/// Start the engine on the config's id, namespace, and homes, mcp its host.
pub(crate) fn init_engine() -> Result<(), String> {
    let cfg = config();
    let params = EngineParams {
        id: cfg.id.clone(),
        namespace: cfg.namespace.clone(),
        work_dir: cfg.work_dir.clone(),
        data_home: data_base_dir(),
        cache_home: cache_base_dir(),
    };
    engine::init(params, Box::new(McpHost)).map_err(|e| e.to_string())
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
        reject_reserved_model(&model)?;
        let channel = channel_handle();
        match channel.status().phase {
            ChannelPhase::Closed => {
                return Err(ChannelSendError::NotOpen.message().to_string());
            }
            ChannelPhase::Open => {
                channel.close(CLOSE_UNVERIFIED_EMIT, UNVERIFIED_EMIT_REASON);
                return Err(ChannelSendError::NotVerified.message().to_string());
            }
            ChannelPhase::Verified => {}
        }

        match channel.record_send(&from) {
            SpamVerdict::Clear => {}
            SpamVerdict::Warn { hits, window_secs } => {
                let rate = channel.thresholds().warn_rate;
                channel.fire_emergency(Emergency::ChannelSpamWarning(
                    ChannelSpamWarningEmergency {
                        origin: from.clone(),
                        hits,
                        window_secs,
                        rate,
                    },
                ));
            }
            SpamVerdict::Stop {
                hits,
                window_secs,
                notify,
            } => {
                let rate = channel.thresholds().error_rate;
                let action = stop();
                if notify {
                    channel.fire_emergency(Emergency::ChannelSpamError(
                        ChannelSpamErrorEmergency {
                            origin: from.clone(),
                            hits,
                            window_secs,
                            rate,
                            action: action.clone(),
                        },
                    ));
                }
                return Err(format!(
                    "channel send rate exceeded: {hits} in {window_secs}s from {from} \
                     (limit {rate}); the channel carries STATE SUMMARIES, not data - \
                     put bulk in `attached`, or write a nuon file",
                ));
            }
        }

        let event_nuon = render_nuon(&event)?;
        let attached_nuon = match &attached {
            Some(value) => Some(render_nuon(value)?),
            None => None,
        };
        let id = mint_msg_id(
            channel.nonce_gen(),
            &from,
            &model,
            &event_nuon,
            attached_nuon.as_deref(),
        );

        let attached_name = match &attached_nuon {
            Some(nuon) => Some(write_inbox_file(&channel, &format!("{id}.nuon"), nuon)?),
            None => None,
        };

        let wire_event = if event_overflows(&event_nuon) {
            let rel = write_inbox_file(&channel, &format!("{id}.event.nuon"), &event_nuon)?;
            event_spill_pointer(&rel, event_nuon.len())
        } else {
            event
        };

        let line = render_packet(id, &from, &model, &wire_event, attached_name.as_deref())?;
        channel.emit(line).map_err(|e| e.message().to_string())?;
        Ok(id.to_string())
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
        let id = mint_remote_id(&model, &event_nuon);
        let (wire_event_nuon, payloads) = spill_remote_event(event_nuon, payloads)?;
        find_link_send(&mcp_nom, id.clone(), model, wire_event_nuon, payloads)?;
        Ok(id)
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

/// The `mcp/` reservation is host-origin only; an agent send picks another path.
fn reject_reserved_model(model: &str) -> Result<(), String> {
    if model.starts_with(MCP_RESERVED_PREFIX) {
        return Err(format!(
            "`{MCP_RESERVED_PREFIX}` is reserved for host-originated models; \
             choose a model path outside it",
        ));
    }
    Ok(())
}

/// Write `nuon` to `inbox/<name>` and return the name the wire carries.
fn write_inbox_file(
    channel: &ChannelHandle,
    name: &str,
    nuon: &str,
) -> Result<String, String> {
    let dir = channel.inbox().ok_or_else(|| {
        "the channel has no inbox; run `interface channel open` first".to_string()
    })?;
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let path = dir.join(name);
    fs::write(&path, nuon.as_bytes()).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(name.to_string())
}

/// Read each `{src, dest}` row into `(dest, bytes)`, its dest checked first.
fn read_attachments(value: &nu::Value) -> Result<Vec<(String, Vec<u8>)>, String> {
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
        out.push((dest.to_string(), bytes));
    }
    Ok(out)
}

/// Mint the send's message id off the shared channel NonceGen (a unique base62).
fn mint_remote_id(
    model: &str,
    event_nuon: &str,
) -> String {
    mint_msg_id(channel_handle().nonce_gen(), "remote/send", model, event_nuon, None).to_string()
}

/// Spill an event the receiver's cap would truncate into a transferred file.
///
/// Returns the wire event's NUON, a pointer when it spilled, and the payloads,
/// the spill among them.
fn spill_remote_event(
    event_nuon: String,
    mut payloads: Vec<(String, Vec<u8>)>,
) -> Result<(String, Vec<(String, Vec<u8>)>), String> {
    if !event_overflows(&event_nuon) {
        return Ok((event_nuon, payloads));
    }
    let pointer_nuon = render_nuon(&event_spill_pointer(EVENT_SPILL_DEST, event_nuon.len()))?;
    payloads.push((EVENT_SPILL_DEST.to_string(), event_nuon.into_bytes()));
    Ok((pointer_nuon, payloads))
}

/// The whole config as a nu record, mirroring the TOML's two tables.
fn config_record() -> nu::Value {
    let span = nu::Span::unknown();
    let cfg = config();
    let spam = channel_handle().thresholds();
    let supervisor = effective_supervisor();

    let mut channel = nu::Record::new();
    channel.insert(
        "port",
        match cfg.channel.port {
            Some(port) => nu::Value::int(port as i64, span),
            None => nu::Value::nothing(span),
        },
    );
    channel.insert(
        "cert_dir",
        nu::Value::string(cfg.channel.cert_dir.to_string_lossy().into_owned(), span),
    );
    channel.insert(
        "spam_warn_window_secs",
        nu::Value::int(spam.warn_window.as_secs() as i64, span),
    );
    channel.insert("spam_warn_rate", nu::Value::int(spam.warn_rate as i64, span));
    channel.insert(
        "spam_error_window_secs",
        nu::Value::int(spam.error_window.as_secs() as i64, span),
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

    let mut root = nu::Record::new();
    root.insert("channel", nu::Value::record(channel, span));
    root.insert("supervisor", nu::Value::record(sup, span));
    nu::Value::record(root, span)
}
