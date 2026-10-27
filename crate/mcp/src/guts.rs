//! In-process test-support harness driving the real tool handlers.

use crate::*;

use std::sync::Once;

static TEST_CONFIG: Once = Once::new();

const INPROC_PREFIX: &str = "grammar_inproc_";

/// Remove the temp namespaces left by test processes that have since exited.
#[cfg(target_os = "linux")]
fn sweep_dead_test_namespaces(temp: &std::path::Path) {
    let Ok(entries) = fs::read_dir(temp) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name
            .strip_prefix(INPROC_PREFIX)
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == process::id() || PathBuf::from(format!("/proc/{pid}")).exists() {
            continue;
        }
        let _ = fs::remove_dir_all(entry.path());
    }
}

#[cfg(not(target_os = "linux"))]
fn sweep_dead_test_namespaces(_temp: &std::path::Path) {}

/// Point the XDG roots at a per-binary temp dir and seed CONFIG, once.
fn ensure_test_config() {
    TEST_CONFIG.call_once(|| {
        let temp = std::env::temp_dir();
        sweep_dead_test_namespaces(&temp);
        let root = temp.join(format!("{INPROC_PREFIX}{}", process::id()));
        let _ = fs::create_dir_all(root.join("data"));
        let _ = fs::create_dir_all(root.join("cache"));
        unsafe {
            std::env::set_var("XDG_DATA_HOME", root.join("data"));
            std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
        }
        let mut config = Config::default();
        config.id = "test".to_string();
        config.namespace = "default".to_string();
        config.work_dir = root.join("work");
        let _ = CONFIG.set(config);
        init_config_dirs().expect("nushell's config dirs resolve for the tests");
    });
}

/// An in-process handle to the grammar tool surface.
pub struct TestServer {
    rt: tokio::runtime::Runtime,
    nush: NuSh,
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TestServer {
    pub fn new() -> Self {
        ensure_test_config();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(EVAL_STACK_SIZE)
            .build()
            .expect("tokio runtime");
        let locks = rt.block_on(ensure_substrate()).expect("ensure_substrate");
        let nush = NuSh::new(
            Arc::new(datum::NonceGenerator::new()),
            locks,
            Arc::new(LintEngine::new()),
        );
        Self { rt, nush }
    }

    fn obj(v: json::Value) -> mcp::JsonObject {
        match v {
            json::Value::Object(m) => m,
            json::Value::Null => mcp::JsonObject::new(),
            other => panic!("expected a JSON object, got {other}"),
        }
    }

    /// A tool's envelope as JSON, read from its NUON text or its structured content.
    fn envelope(r: Result<mcp::CallToolResult, mcp::ErrorData>) -> json::Value {
        let ctr = r.unwrap_or_else(|e| panic!("unexpected rmcp-boundary error: {e:?}"));
        if let Some(mcp::ContentBlock::Text(text)) = ctr.content.first() {
            return nuon_to_json(&text.text);
        }
        ctr.structured_content.unwrap_or(json::Value::Null)
    }

    /// `nu` on a definition and its data's NUON text.
    pub fn nu(
        &self,
        definition: &str,
        data: &str,
    ) -> json::Value {
        self.nu_timeout(definition, data, None)
    }

    /// `nu` with a timeout, a Nu duration's text.
    pub fn nu_timeout(
        &self,
        definition: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> json::Value {
        let p = NuParams {
            definition: definition.to_string(),
            data: data.to_string(),
            timeout: timeout.map(str::to_string),
        };
        Self::envelope(self.rt.block_on(self.nush.nu(mcp::Parameters(p))))
    }

    /// `rerun` of an earlier `nu` call with fresh data.
    pub fn rerun(
        &self,
        nonce: &str,
        data: &str,
    ) -> json::Value {
        self.rerun_timeout(nonce, data, None)
    }

    /// `rerun` with a timeout, a Nu duration's text.
    pub fn rerun_timeout(
        &self,
        nonce: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> json::Value {
        let p = RerunParams {
            nonce: nonce.to_string(),
            data: data.to_string(),
            timeout: timeout.map(str::to_string),
        };
        Self::envelope(self.rt.block_on(self.nush.rerun(mcp::Parameters(p))))
    }

    pub fn call(&self, namepath: &str, args: json::Value) -> json::Value {
        let p = CallParams {
            namepath: namepath.to_string(),
            args: Self::obj(args),
            timeout_ms: None,
        };
        Self::envelope(self.rt.block_on(self.nush.call(mcp::Parameters(p))))
    }

    pub fn commit(&self, namepath: &str) -> json::Value {
        let p = CommitParams {
            namepath: namepath.to_string(),
        };
        Self::envelope(self.rt.block_on(self.nush.commit(mcp::Parameters(p))))
    }

    pub fn equip(&self, action: &str, source_dir: &str) -> json::Value {
        let p = EquipParams {
            action: action.to_string(),
            source_dir: source_dir.to_string(),
        };
        Self::envelope(self.rt.block_on(self.nush.equip(mcp::Parameters(p))))
    }

    /// `equip(new)` at `root`'s source for `name`, returning its rig source.
    pub fn new_rig(
        &self,
        root: &std::path::Path,
        name: &str,
    ) -> std::path::PathBuf {
        let source = equip_source(root, name);
        let env = self.equip("new", &source.to_string_lossy());
        assert!(!has_error(&env), "equip(new) for {name} failed: {env}");
        source.join("rig")
    }

    pub fn scaffold(&self, namepaths: &[&str]) -> json::Value {
        let p = NewParams {
            namepaths: namepaths.iter().map(|s| s.to_string()).collect(),
        };
        Self::envelope(self.rt.block_on(self.nush.scaffold(mcp::Parameters(p))))
    }

    pub fn inspect(&self, namepath: &str) -> json::Value {
        let p = InspectParams {
            namepath: namepath.to_string(),
        };
        Self::envelope(self.rt.block_on(self.nush.inspect(mcp::Parameters(p))))
    }

    pub fn info(&self) -> json::Value {
        self.info_as(&[])
    }

    /// `info()` rendered as if the given purviews were in view.
    pub fn info_as(
        &self,
        purviews: &[&str],
    ) -> json::Value {
        let p = InfoParams {
            purviews: purviews.iter().map(|s| s.to_string()).collect(),
        };
        Self::envelope(self.rt.block_on(self.nush.info(mcp::Parameters(p))))
    }

    pub fn purviews(&self) -> json::Value {
        Self::envelope(
            self.rt
                .block_on(self.nush.purviews(mcp::Parameters(PurviewsParams {}))),
        )
    }

    /// Set a purview's namepath patterns; an EMPTY `namepaths` deletes it.
    pub fn purview_configure(
        &self,
        purview: &str,
        namepaths: &[&str],
    ) -> json::Value {
        let p = PurviewConfigureParams {
            purview: purview.to_string(),
            namepaths: namepaths.iter().map(|s| s.to_string()).collect(),
        };
        Self::envelope(
            self.rt
                .block_on(self.nush.purview_configure(mcp::Parameters(p))),
        )
    }

    pub fn purview_extend(
        &self,
        purviews: &[&str],
    ) -> json::Value {
        let p = PurviewExtendParams {
            purviews: purviews.iter().map(|s| s.to_string()).collect(),
        };
        Self::envelope(self.rt.block_on(self.nush.purview_extend(mcp::Parameters(p))))
    }

    pub fn purview(
        &self,
        purviews: &[&str],
    ) -> json::Value {
        let p = PurviewParams {
            purviews: purviews.iter().map(|s| (*s).to_string()).collect(),
        };
        Self::envelope(self.rt.block_on(self.nush.purview(mcp::Parameters(p))))
    }

    /// Where this namespace's purview table lands.
    pub fn purviews_path(&self) -> std::path::PathBuf {
        crate::purviews_path()
    }

    pub fn processes(&self) -> json::Value {
        Self::envelope(
            self.rt
                .block_on(self.nush.processes(mcp::Parameters(ProcessesParams {}))),
        )
    }

    /// `kill` is a no-return tool: the envelope is JSON null on success.
    pub fn kill(&self, nonce: &str) -> json::Value {
        let p = KillParams {
            nonce: nonce.to_string(),
        };
        Self::envelope(self.rt.block_on(self.nush.kill(mcp::Parameters(p))))
    }

    /// `channel_verified` is a no-return tool; the envelope is null.
    pub fn channel_verified(&self) -> json::Value {
        Self::envelope(
            self.rt
                .block_on(self.nush.channel_verified(mcp::Parameters(ChannelVerifiedParams {}))),
        )
    }

    /// A PARTIAL update; pass `None` for anything that should not move.
    pub fn config_channel(
        &self,
        warn_window_secs: Option<u64>,
        warn_rate: Option<u32>,
        error_window_secs: Option<u64>,
        error_rate: Option<u32>,
    ) -> json::Value {
        let p = ConfigChannelParams {
            spam_warn_window_secs: warn_window_secs,
            spam_warn_rate: warn_rate,
            spam_error_window_secs: error_window_secs,
            spam_error_rate: error_rate,
        };
        Self::envelope(self.rt.block_on(self.nush.config_channel(mcp::Parameters(p))))
    }

    /// `channel_close` is a no-return tool; closing a closed channel succeeds.
    pub fn channel_close(&self) -> json::Value {
        Self::envelope(
            self.rt
                .block_on(self.nush.channel_close(mcp::Parameters(ChannelCloseParams {}))),
        )
    }

    /// The in-process equip git repo dir.
    pub fn equip_root(&self) -> std::path::PathBuf {
        crate::equip_root()
    }

    /// The per-call log dir of a `nu` call's nonce.
    pub fn nu_log_dir(&self, nonce: &str) -> std::path::PathBuf {
        crate::definition_file(nonce)
            .parent()
            .expect("a definition path always has a parent dir")
            .to_path_buf()
    }

    /// The committed rig of an equip by its compound `author/name`.
    pub fn rig_dir(&self, name: &str) -> std::path::PathBuf {
        crate::rig_dir(name)
    }

    /// The committed lib of an equip by its compound `author/name`.
    pub fn lib_dir(&self, name: &str) -> std::path::PathBuf {
        crate::lib_dir(name)
    }

    /// A rig's committed index, decoded into the JSON shape assertions use.
    pub fn rig_index(&self, name: &str) -> json::Value {
        let path = crate::server::rig::rig_meta_path(name);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read index {}: {e}", path.display()));
        let index = crate::server::rig::index_from_nuon(&text)
            .unwrap_or_else(|e| panic!("decode index {}: {e}", path.display()));
        json::to_value(&index).expect("index serializes")
    }
}

/// Drop every config pin.
pub fn clear_config_pins() {
    crate::clear_pins();
}

/// A NUON text as JSON, through the converter `to json` uses.
pub fn nuon_to_json(text: &str) -> json::Value {
    let value = nu::from_nuon(text, None).unwrap_or_else(|e| panic!("NUON `{text}`: {e}"));
    let converted =
        nu::JsonValue::from_value(value).unwrap_or_else(|e| panic!("NUON `{text}` as JSON: {e}"));
    json::to_value(&converted).expect("a JSON value serializes")
}

/// True when the envelope is the error shape (`{ error: ... }`).
pub fn has_error(env: &json::Value) -> bool {
    env.get("error").is_some()
}

/// The first error diagnostic's `kind`, if any.
pub fn error_kind(env: &json::Value) -> Option<&str> {
    env.get("error")?
        .get("errors")?
        .as_array()?
        .first()?
        .get("kind")?
        .as_str()
}

/// Every error/warning diagnostic `kind` in the envelope.
pub fn error_kinds(env: &json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(err) = env.get("error") {
        for bucket in ["errors", "warnings"] {
            if let Some(arr) = err.get(bucket).and_then(|v| v.as_array()) {
                for d in arr {
                    if let Some(k) = d.get("kind").and_then(|k| k.as_str()) {
                        out.push(k.to_string());
                    }
                }
            }
        }
    }
    out
}

pub fn has_kind(env: &json::Value, kind: &str) -> bool {
    error_kinds(env).iter().any(|k| k == kind)
}

/// Every ERROR-bucket diagnostic `message` in the envelope.
pub fn error_messages(env: &json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = env
        .get("error")
        .and_then(|e| e.get("errors"))
        .and_then(|v| v.as_array())
    {
        for d in arr {
            if let Some(m) = d.get("message").and_then(|m| m.as_str()) {
                out.push(m.to_string());
            }
        }
    }
    out
}

/// The whole error object rendered to a string.
pub fn error_text(env: &json::Value) -> String {
    env.get("error").map(|e| e.to_string()).unwrap_or_default()
}

/// One rig's slice of an `info()` signature block.
pub fn rig_block(
    signatures: &str,
    leaf: &str,
) -> String {
    let head = format!(" {leaf}");
    let mut out: Vec<&str> = Vec::new();
    for line in signatures.lines() {
        if out.is_empty() {
            if line == head || line.starts_with(&format!("{head} ")) {
                out.push(line);
            }
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        if indent <= 1 {
            break;
        }
        out.push(line);
    }
    assert!(
        !out.is_empty(),
        "rig `{leaf}` is absent from the block:\n{signatures}",
    );
    format!("{}\n", out.join("\n"))
}

/// An equip's source directory under `root`: `root/equip/<name>`.
pub fn equip_source(
    root: &std::path::Path,
    name: &str,
) -> std::path::PathBuf {
    root.join("equip").join(name)
}

pub fn write_source(dir: &std::path::Path, rel: &str, contents: &str) {
    let target = dir.join(rel);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(&target, contents).expect("write source");
}

pub fn write_tree(dir: &std::path::Path, files: &[(&str, &str)]) {
    for (rel, contents) in files {
        write_source(dir, rel, contents);
    }
}

pub fn valid_function_source(args_schema: &str, result_schema: &str, body: &str) -> String {
    format!(
        "export def main [args: record<{args_schema}>]: nothing -> record<{result_schema}> {{\n{body}\n}}\n",
    )
}

fn to_obj(v: json::Value) -> mcp::JsonObject {
    match v {
        json::Value::Object(m) => m,
        json::Value::Null => mcp::JsonObject::new(),
        other => panic!("expected a JSON object, got {other}"),
    }
}

/// Lint an agent body, returning each Diagnostic as its wire JSON.
pub fn lint_body(args_type: &str, body: &str) -> Vec<json::Value> {
    ensure_test_config();
    let engine = ParseEngine::new_full();
    crate::lint_body(&engine, args_type, body)
        .iter()
        .map(|d| json::to_value(d).expect("diagnostic serializes"))
        .collect()
}

/// The execute contract's verdict on a definition: its summary, or its rows.
pub fn analyze_definition(definition: &str) -> Result<json::Value, Vec<json::Value>> {
    ensure_test_config();
    let engine = ParseEngine::new_full();
    match crate::analyze_definition(&engine, definition, true) {
        Ok(def) => Ok(serde_json::json!({
            "lane": def.lane.as_str(),
            "data_type": typedef_text(&def.data_type),
            "result_type": typedef_text(&def.result_type),
            "takes_timeout": def.takes_timeout,
        })),
        Err(rows) => Err(rows
            .iter()
            .map(|d| json::to_value(d).expect("diagnostic serializes"))
            .collect()),
    }
}

/// Are two `current()` handles off ONE `LintEngine` the same instance?
pub fn lint_engine_reuses_until_the_registry_moves() -> bool {
    ensure_test_config();
    let engine = LintEngine::new();
    let first = engine.current();
    let second = engine.current();
    Arc::ptr_eq(&first, &second)
}

/// True if `src` parses clean on a full-shell ParseEngine.
pub fn parses_clean(src: &str) -> bool {
    ensure_test_config();
    let engine = ParseEngine::new_full();
    let mut ws = nu::StateWorkingSet::new(engine.engine_state());
    let _ = nu::parse(&mut ws, Some("golden.nu"), src.as_bytes(), false);
    ws.parse_errors.is_empty()
}

/// The synthesized nu() source for a definition and its literals.
pub fn build_nu_source(
    definition: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> String {
    crate::build_nu_source(definition, data_literal, timeout_literal)
}

/// The synthesized call() source.
pub fn build_call_source(
    rig: &str,
    module_path: &str,
    name: &str,
    args: json::Value,
) -> String {
    crate::build_call_source(rig, module_path, name, &to_obj(args))
}
