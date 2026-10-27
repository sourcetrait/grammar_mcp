use crate::*;

/// Write an `execute` call's definition beside its logs, the handle `rerun` reads.
fn write_definition(
    log_dir: &std::path::Path,
    definition: &str,
) {
    let path = log_dir.join(DEFINITION_FILE);
    if let Err(e) = fs::write(&path, definition.as_bytes()) {
        eprintln!("grammar: definition cache write failed at {}: {e}", path.display());
    }
}

pub struct NuSh {
    pub(crate) stateful_engine: Arc<tk::AsyncMutex<Option<StatefulEngine>>>,
    /// The stateless eval executor; hands each eval a ready clone.
    pub(crate) executor: Arc<Executor>,
    /// Host-owned environment-wide jobs table shared into every eval.
    pub(crate) env_jobs: Arc<std::sync::Mutex<nu::Jobs>>,
    pub(crate) nonce_gen: Arc<datum::NonceGenerator>,
    /// This host process's identity, minted once at construction.
    pub(crate) mcp_nom: datum::NomPair,
    pub(crate) rig_locks: Arc<RigLocks>,
    /// The lint and rig-validator engine, taken through `current()`.
    pub(crate) lint_engine: Arc<LintEngine>,
    /// The engine an interface command parses on: its commands, nothing else.
    pub(crate) interface_engine: Arc<nu::EngineState>,
    /// The resource registry, keyed by nonce string.
    pub(crate) in_flight: Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>,
    /// The hang-detection registry the watchdog scans.
    pub(crate) hung_watch: HungRegistry,
    /// The host's single packet channel; present but CLOSED at startup.
    pub(crate) channel: Arc<ChannelHandle>,
    /// Serializes `channel_open`'s decide-then-start across its await.
    pub(crate) channel_open_lock: Arc<tk::AsyncMutex<()>>,
    /// Which purview ids this host has in view; SESSION-resident.
    pub(crate) current_purview: Arc<CurrentPurview>,
    /// The process-wide registry of established remote links, keyed by alias.
    pub(crate) remote_links: Arc<std::sync::Mutex<HashMap<String, RemoteLinkEntry>>>,
    pub(crate) tool_router: mcp::ToolRouter<NuSh>,
}

/// One open remote link plus the peer address, for the registry and listing.
pub(crate) struct RemoteLinkEntry {
    pub handle: RemoteLinkHandle,
    pub addr: std::net::SocketAddr,
}

pub(crate) const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Pre-clone depth N for the stateless executor's ready buffer.
pub(crate) const READY_POOL_TARGET: usize = 1;

pub(crate) struct InFlightEntry {
    pub tool: &'static str,
    pub started_at: u64,
    pub args: serde_json::Value,
    pub kind: InFlightKind,
    /// This eval's interrupt flag - the `Signals` handle its thread polls.
    pub cancel: Arc<AtomicBool>,
    /// This eval's external-child tracker, holding Arc-shared pids.
    pub tracker: nu::ThreadJob,
    /// Liveness: the eval thread flips this on exit via a Drop guard.
    pub finished: Arc<AtomicBool>,
}

pub(crate) enum InFlightKind {
    Execute { lane: Lane },
    Rerun { source_nonce: String, lane: Lane },
    Call { path: String },
}

impl InFlightKind {
    /// The lane the call's eval runs on.
    pub(crate) fn lane(&self) -> Lane {
        match self {
            Self::Execute { lane } | Self::Rerun { lane, .. } => *lane,
            Self::Call { .. } => Lane::Stateless,
        }
    }
}

impl NuSh {
    pub(crate) fn new(
        nonce_gen: Arc<datum::NonceGenerator>,
        rig_locks: Arc<RigLocks>,
        lint_engine: Arc<LintEngine>,
        deny: &DenySet,
    ) -> Self {
        nu::CRYPTO_PROVIDER.default();
        let env_jobs = Arc::new(std::sync::Mutex::new(nu::Jobs::default()));
        let executor = Arc::new(Executor::new(env_jobs.clone(), READY_POOL_TARGET));
        let mcp_nom = datum::Nom::generate(&nonce_gen).into_pair();
        Self {
            stateful_engine: Arc::new(tk::AsyncMutex::new(None)),
            executor,
            env_jobs,
            nonce_gen,
            mcp_nom,
            rig_locks,
            lint_engine,
            interface_engine: Arc::new(interface_engine(deny)),
            in_flight: Arc::new(tk::AsyncMutex::new(HashMap::new())),
            hung_watch: Arc::new(std::sync::Mutex::new(HashMap::new())),
            channel: channel_handle(),
            channel_open_lock: Arc::new(tk::AsyncMutex::new(())),
            current_purview: Arc::new(CurrentPurview::new()),
            remote_links: remote_links(),
            tool_router: Self::tool_router(),
        }
    }
}

impl NuSh {
    /// A call's nonce, minted first from its tool and its parameters.
    pub(crate) fn mint<P: ser::Serialize>(
        &self,
        tool: &str,
        params: &P,
    ) -> datum::NoncePair {
        let payload = json::to_vec(params).unwrap_or_default();
        self.nonce_gen.generate_with(&(tool, payload)).into_pair()
    }
}

/// An envelope, `nonce` first and one more field, as the result's one NUON text.
pub(crate) fn envelope_text(
    nonce: datum::Nonce,
    field: &str,
    value: nu::Value,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    let span = nu::Span::unknown();
    let mut envelope = nu::Record::new();
    envelope.insert("nonce", nu::Value::string(nonce.to_string(), span));
    envelope.insert(field, value);
    let text = render_nuon(&nu::Value::record(envelope, span))
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope render: {e}"), None))?;
    Ok(mcp::CallToolResult::success(vec![mcp::ContentBlock::text(text)]))
}

/// A tool's record as a nu value, through the serde bridge.
pub(crate) fn serde_to_nu<T: ser::Serialize>(value: &T) -> Result<nu::Value, mcp::ErrorData> {
    let json = json::to_value(value)
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope serialize: {e}"), None))?;
    Ok(json_value_to_nu_value(&json))
}

/// A success, `{nonce, result}`: the result a nu value the tool built.
pub(crate) fn success_value(
    nonce: datum::Nonce,
    result: nu::Value,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    envelope_text(nonce, "result", result)
}

/// A success, `{nonce, result}`: the result a tool's record.
pub(crate) fn success<T: ser::Serialize>(
    nonce: datum::Nonce,
    result: &T,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    success_value(nonce, serde_to_nu(result)?)
}

/// A success with nothing to answer, `{nonce, result: null}`.
pub(crate) fn success_void(nonce: datum::Nonce) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    success_value(nonce, nu::Value::nothing(nu::Span::unknown()))
}

pub(crate) struct DispatchOutcome {
    pub(crate) nonce: datum::Nonce,
    pub(crate) value: nu::Value,
}

pub(crate) struct DispatchError {
    pub(crate) error: GrammarMcpError,
    pub(crate) nonce: datum::Nonce,
}

pub(crate) async fn dispatch_pooled(
    executor: &Arc<Executor>,
    in_flight: &Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>,
    hung_watch: &HungRegistry,
    log_kind: CacheKind,
    nonce: datum::NoncePair,
    source: String,
    tool_name: &'static str,
    args_json: serde_json::Value,
    kind: InFlightKind,
    definition: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<DispatchOutcome, DispatchError> {
    let log_dir = cache_dir(log_kind, &nonce);
    fs::create_dir_all(&log_dir).map_err(|e| DispatchError {
        error: GrammarMcpError::Internal {
            phase: "dispatch_pooled::create_log_dir".to_string(),
            reason: format!("create_dir_all {}: {e}", log_dir.display()),
        },
        nonce: nonce.to_nonce(),
    })?;
    if let Some(definition) = definition {
        write_definition(&log_dir, definition);
    }
    executor.refresh_base_if_stale();
    let effective_timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let budget = tk::TkDuration::from_millis(effective_timeout);
    let dispatched_at = Instant::now();
    let semaphore = executor.semaphore();
    let permit = match tk::timeout(budget, semaphore.clone().acquire_owned()).await {
        Ok(Ok(permit)) => permit,
        Ok(Err(e)) => {
            return Err(DispatchError {
                error: GrammarMcpError::ThreadDispatch {
                    reason: format!("eval semaphore: {e}"),
                },
                nonce: nonce.to_nonce(),
            });
        }
        Err(_) => {
            let cap = eval_concurrency_cap();
            let held = cap.saturating_sub(semaphore.available_permits());
            return Err(DispatchError {
                error: GrammarMcpError::ThreadDispatch {
                    reason: format!(
                        "no eval permit within {effective_timeout} ms; {held} of {cap} permits held",
                    ),
                },
                nonce: nonce.to_nonce(),
            });
        }
    };
    let mut engine = executor.take_clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let started_at = now_millis();
    let finished = Arc::new(AtomicBool::new(false));
    let tracker = make_tracker(cancel.clone());
    engine.current_job.background_thread_job = Some(tracker.clone());
    register_in_flight(
        in_flight,
        &nonce,
        tool_name,
        args_json,
        kind,
        started_at,
        cancel.clone(),
        tracker.clone(),
        finished.clone(),
    )
    .await;
    let _flight_cleanup = InFlightCleanup {
        map: in_flight.clone(),
        key: nonce.as_str().to_string(),
    };
    let eval_fut = eval_stateless(
        engine,
        cancel.clone(),
        log_dir,
        source,
        nonce.as_str().to_string(),
        permit,
        finished.clone(),
    );
    let remaining = budget.saturating_sub(dispatched_at.elapsed());
    let timed = tk::timeout(remaining, eval_fut).await;
    match timed {
        Ok(Ok(value)) => Ok(DispatchOutcome {
            nonce: nonce.to_nonce(),
            value,
        }),
        Ok(Err(failure)) => Err(DispatchError {
            error: failure.into(),
            nonce: nonce.to_nonce(),
        }),
        Err(_) => {
            cancel.store(true, Ordering::SeqCst);
            tree_kill(&tracker.collect_pids());
            register_hung(
                hung_watch,
                HungWatch {
                    nonce: nonce.as_str().to_string(),
                    tool: tool_name,
                    lane: Lane::Stateless,
                    started_at,
                    cancelled_at: now_millis(),
                    finished: finished.clone(),
                },
            );
            Err(DispatchError {
                error: GrammarMcpError::ThreadTimeout {
                    timeout_ms: effective_timeout,
                },
                nonce: nonce.to_nonce(),
            })
        }
    }
}

pub(crate) async fn dispatch_stateful(
    stateful: &Arc<tk::AsyncMutex<Option<StatefulEngine>>>,
    env_jobs: &Arc<std::sync::Mutex<nu::Jobs>>,
    in_flight: &Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>,
    hung_watch: &HungRegistry,
    nonce: datum::NoncePair,
    source: String,
    tool_name: &'static str,
    args_json: serde_json::Value,
    kind: InFlightKind,
    definition: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<DispatchOutcome, DispatchError> {
    let log_dir = cache_dir(CacheKind::Execute, &nonce);
    fs::create_dir_all(&log_dir).map_err(|e| DispatchError {
        error: GrammarMcpError::Internal {
            phase: "dispatch_stateful::create_log_dir".to_string(),
            reason: format!("create_dir_all {}: {e}", log_dir.display()),
        },
        nonce: nonce.to_nonce(),
    })?;
    if let Some(definition) = definition {
        write_definition(&log_dir, definition);
    }
    let engine = {
        let mut guard = stateful.lock().await;
        if guard.is_none() {
            *guard = Some(StatefulEngine::spawn(env_jobs.clone()));
        }
        guard
            .as_ref()
            .expect("stateful engine present after spawn")
            .clone()
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let tracker = make_tracker(cancel.clone());
    let started_at = now_millis();
    let finished = Arc::new(AtomicBool::new(false));
    register_in_flight(
        in_flight,
        &nonce,
        tool_name,
        args_json,
        kind,
        started_at,
        cancel.clone(),
        tracker.clone(),
        finished.clone(),
    )
    .await;
    let _flight_cleanup = InFlightCleanup {
        map: in_flight.clone(),
        key: nonce.as_str().to_string(),
    };
    let effective_timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let eval_fut = engine.eval(
        log_dir,
        source,
        nonce.as_str().to_string(),
        cancel.clone(),
        tracker.clone(),
        finished.clone(),
    );
    let timed = tk::timeout(tk::TkDuration::from_millis(effective_timeout), eval_fut).await;
    match timed {
        Ok(Ok(value)) => Ok(DispatchOutcome {
            nonce: nonce.to_nonce(),
            value,
        }),
        Ok(Err(failure)) => Err(DispatchError {
            error: failure.into(),
            nonce: nonce.to_nonce(),
        }),
        Err(_) => {
            cancel.store(true, Ordering::SeqCst);
            tree_kill(&tracker.collect_pids());
            register_hung(
                hung_watch,
                HungWatch {
                    nonce: nonce.as_str().to_string(),
                    tool: tool_name,
                    lane: Lane::Stateful,
                    started_at,
                    cancelled_at: now_millis(),
                    finished: finished.clone(),
                },
            );
            Err(DispatchError {
                error: GrammarMcpError::ThreadTimeout {
                    timeout_ms: effective_timeout,
                },
                nonce: nonce.to_nonce(),
            })
        }
    }
}

async fn register_in_flight(
    in_flight: &Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>,
    nonce: &datum::NoncePair,
    tool_name: &'static str,
    args_json: serde_json::Value,
    kind: InFlightKind,
    started_at: u64,
    cancel: Arc<AtomicBool>,
    tracker: nu::ThreadJob,
    finished: Arc<AtomicBool>,
) {
    let mut map = in_flight.lock().await;
    map.insert(
        nonce.as_str().to_string(),
        InFlightEntry {
            tool: tool_name,
            started_at,
            args: args_json,
            kind,
            cancel,
            tracker,
            finished,
        },
    );
}

struct InFlightCleanup {
    map: Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>,
    key: String,
}

impl Drop for InFlightCleanup {
    fn drop(&mut self) {
        let map = self.map.clone();
        let key = std::mem::take(&mut self.key);
        tk::spawn(async move {
            let mut g = map.lock().await;
            g.remove(&key);
        });
    }
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Clean-shutdown teardown: cancel every in-flight eval and reap its tree.
pub(crate) async fn teardown_all_in_flight(
    in_flight: &Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>,
    env_jobs: &Arc<std::sync::Mutex<nu::Jobs>>,
) {
    let pids: Vec<u32> = {
        let map = in_flight.lock().await;
        let mut all = Vec::new();
        for entry in map.values() {
            entry.cancel.store(true, Ordering::SeqCst);
            all.extend(entry.tracker.collect_pids());
        }
        all
    };
    tree_kill(&pids);
    sweep_env_jobs(env_jobs);
    kill_plugin_subprocesses();
}

/// Kill every background job in the host-owned table and reap their trees.
pub(crate) fn sweep_env_jobs(env_jobs: &Arc<std::sync::Mutex<nu::Jobs>>) {
    let tracked: Vec<u32> = {
        let mut jobs = env_jobs.lock().unwrap_or_else(|e| e.into_inner());
        let tracked = jobs
            .iter()
            .filter_map(|(_, job)| match job {
                nu::Job::Thread(thread_job) => Some(thread_job.collect_pids()),
                _ => None,
            })
            .flatten()
            .collect();
        let _ = jobs.kill_all();
        tracked
    };
    tree_kill(&tracked);
}
