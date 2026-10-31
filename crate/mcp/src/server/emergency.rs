//! The controller's emergency policy: each logged, then announced on the Channel.
use crate::*;

/// The reserved model path an emergency rides under on the channel.
pub(crate) fn emergency_model(emergency: &Emergency) -> &'static str {
    match emergency.kind() {
        EmergencyKind::HungEngineThread => "mcp/supervisor/HungEngineThread",
        EmergencyKind::CpuWarning => "mcp/supervisor/CpuWarning",
        EmergencyKind::RamWarning => "mcp/supervisor/RamWarning",
        EmergencyKind::VramWarning => "mcp/supervisor/VramWarning",
        EmergencyKind::BackgroundJobsWarning => "mcp/supervisor/BackgroundJobsWarning",
        EmergencyKind::DiskWarning => "mcp/supervisor/DiskWarning",
        EmergencyKind::ChannelSpamWarning => "mcp/channel/spam/Warning",
        EmergencyKind::ChannelSpamError => "mcp/channel/spam/Error",
        EmergencyKind::Critical => "mcp/supervisor/Critical",
    }
}

/// An emergency's own fields, without the envelope the log adds.
fn emergency_record(emergency: &Emergency) -> nu::Record {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    match emergency {
        Emergency::HungEngineThread(h) => {
            r.insert("nonce", nu::Value::string(h.nonce.clone(), span));
            r.insert("lane", nu::Value::string(h.lane.clone(), span));
            r.insert("tool", nu::Value::string(h.tool.clone(), span));
            r.insert("started_at", nu::Value::int(h.started_at as i64, span));
            r.insert("cancelled_at", nu::Value::int(h.cancelled_at as i64, span));
            r.insert("hung_ms", nu::Value::int(h.hung_ms as i64, span));
            r.insert("pool_held", nu::Value::int(h.pool_held as i64, span));
            r.insert("pool_cap", nu::Value::int(h.pool_cap as i64, span));
        }
        Emergency::CpuWarning(c) => {
            r.insert("cpu_pct", nu::Value::float(c.cpu_pct, span));
            r.insert("sample_ms", nu::Value::int(c.sample_ms as i64, span));
        }
        Emergency::RamWarning(m) => {
            r.insert("rss_kb", nu::Value::int(m.rss_kb as i64, span));
        }
        Emergency::VramWarning(v) => {
            r.insert("used_mib", nu::Value::int(v.used_mib as i64, span));
            r.insert("total_mib", nu::Value::int(v.total_mib as i64, span));
        }
        Emergency::BackgroundJobsWarning(b) => {
            r.insert("job_count", nu::Value::int(b.job_count as i64, span));
        }
        Emergency::DiskWarning(d) => {
            r.insert("mount", nu::Value::string(d.mount.clone(), span));
            r.insert("used_pct", nu::Value::int(d.used_pct as i64, span));
            r.insert("threshold_pct", nu::Value::int(d.threshold_pct as i64, span));
        }
        Emergency::ChannelSpamWarning(w) => {
            r.insert("origin", nu::Value::string(w.origin.clone(), span));
            r.insert("hits", nu::Value::int(w.hits as i64, span));
            r.insert("window_secs", nu::Value::int(w.window_secs as i64, span));
            r.insert("rate", nu::Value::int(w.rate as i64, span));
        }
        Emergency::ChannelSpamError(e) => {
            r.insert("origin", nu::Value::string(e.origin.clone(), span));
            r.insert("hits", nu::Value::int(e.hits as i64, span));
            r.insert("window_secs", nu::Value::int(e.window_secs as i64, span));
            r.insert("rate", nu::Value::int(e.rate as i64, span));
            r.insert("action", nu::Value::string(e.action.clone(), span));
        }
        Emergency::Critical(c) => {
            r.insert("reason", nu::Value::string(c.reason.clone(), span));
            r.insert("hung", nu::Value::int(c.hung as i64, span));
            r.insert("cap", nu::Value::int(c.cap as i64, span));
        }
    }
    r
}

/// An emergency as a single-line NUON record, the log's line.
pub(crate) fn emergency_line(
    emergency: &Emergency,
    ts: u64,
    mcp_nom: &str,
) -> Result<String, String> {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    r.insert("ts", nu::Value::int(ts as i64, span));
    r.insert("mcp_nom", nu::Value::string(mcp_nom.to_string(), span));
    r.insert("kind", nu::Value::string(emergency.kind().name().to_string(), span));
    for (key, value) in emergency_record(emergency) {
        r.insert(key, value);
    }
    nu::to_nuon(
        &nu::EngineState::new(),
        &nu::Value::record(r, span),
        nu::ToNuonConfig::default(),
    )
    .map(|line| escape_line(&line))
    .map_err(|e| e.to_string())
}

/// The per-process emergency log path.
pub(crate) fn emergency_log_path(mcp_nom: &str) -> PathBuf {
    cache_base_dir().join("log").join(mcp_nom).join("emergency.nuonl")
}

/// Append one already-rendered NUON line, creating the parent dir on demand.
pub(crate) fn append_line(
    path: &std::path::Path,
    line: &str,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

/// The controller's policy over its systems' emergencies: logged, then announced.
#[derive(Clone)]
pub(crate) struct EmergencyPolicy {
    channel: Arc<ChannelHandle>,
    mcp_nom: String,
    log: PathBuf,
}

impl EmergencyPolicy {
    /// The policy of the host `mcp_nom`, logging to `log` and announcing on `channel`.
    pub(crate) fn new(
        channel: Arc<ChannelHandle>,
        mcp_nom: &str,
        log: PathBuf,
    ) -> Self {
        Self {
            channel,
            mcp_nom: mcp_nom.to_string(),
            log,
        }
    }

    /// An emergency logged, then announced on the Channel under its reserved model.
    pub(crate) async fn respond(
        &self,
        emergency: Emergency,
    ) {
        match emergency_line(&emergency, now_millis(), &self.mcp_nom) {
            Ok(line) => {
                if let Err(e) = append_line(&self.log, &line) {
                    eprintln!(
                        "grammar: emergency log append failed at {}: {e}",
                        self.log.display(),
                    );
                }
            }
            Err(e) => eprintln!("grammar: emergency serialize failed: {e}"),
        }
        self.announce(&emergency).await;
    }

    /// Put one emergency on the Channel under its reserved `mcp/` model.
    async fn announce(
        &self,
        emergency: &Emergency,
    ) {
        let event = nu::Value::record(emergency_record(emergency), nu::Span::unknown());
        let Ok(event) = NuVal::try_from_nu(event) else {
            return;
        };
        let emit = EmitRequest {
            from: FROM_MCP.to_string(),
            model: emergency_model(emergency).to_string(),
            event,
            attached: None,
        };
        let _ = self.channel.request(emit).await;
    }
}
