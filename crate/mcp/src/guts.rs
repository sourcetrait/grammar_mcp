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
            &config().deny,
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

    /// A tool's envelope as JSON, read from its NUON text.
    fn envelope(r: Result<mcp::CallToolResult, mcp::ErrorData>) -> json::Value {
        let ctr = r.unwrap_or_else(|e| panic!("unexpected rmcp-boundary error: {e:?}"));
        match ctr.content.first() {
            Some(mcp::ContentBlock::Text(text)) => nuon_to_json(&text.text),
            other => panic!("a result without its envelope text: {other:?}"),
        }
    }

    /// `execute` on a definition and its data's NUON text.
    pub fn execute(
        &self,
        definition: &str,
        data: &str,
    ) -> json::Value {
        self.execute_timeout(definition, data, None)
    }

    /// `execute` with a timeout, a Nu duration's text.
    pub fn execute_timeout(
        &self,
        definition: &str,
        data: &str,
        timeout: Option<&str>,
    ) -> json::Value {
        let p = ExecuteParams {
            definition: definition.to_string(),
            data: data.to_string(),
            timeout: timeout.map(str::to_string),
        };
        Self::envelope(self.rt.block_on(self.nush.execute(mcp::Parameters(p))))
    }

    /// `rerun` of an earlier `execute` call with fresh data.
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

    /// A nonce minted as the interface call would mint one.
    fn interface_nonce<P: ser::Serialize>(&self, p: &P) -> datum::Nonce {
        self.nush.mint(INTERFACE_TOOL, p).to_nonce()
    }

    /// One `interface` command, through the tool's parse and dispatch.
    pub fn interface(&self, command: &str) -> json::Value {
        let p = InterfaceParams {
            command: command.to_string(),
        };
        Self::envelope(self.rt.block_on(self.nush.interface(mcp::Parameters(p))))
    }

    pub fn commit(&self, namepath: &str) -> json::Value {
        let p = CommitParams {
            namepath: namepath.to_string(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.equip_commit(nonce, p)))
    }

    /// An `interface equip` action on a source; an unknown one goes to the parse.
    pub fn equip(&self, action: &str, source_dir: &str) -> json::Value {
        let p = EquipParams {
            source: source_dir.to_string(),
        };
        let nonce = self.interface_nonce(&p);
        let result = match action {
            "new" => self.rt.block_on(self.nush.equip_new(nonce, p)),
            "install" => self.rt.block_on(self.nush.equip_install(nonce, p)),
            "check" => self.rt.block_on(self.nush.equip_check(nonce, p)),
            "uninstall" => self.rt.block_on(self.nush.equip_uninstall(nonce, p)),
            other => return self.interface(&format!("{INTERFACE} equip {other} '{source_dir}'")),
        };
        Self::envelope(result)
    }

    /// `interface equip new` at `root`'s source for `name`; its rig source.
    pub fn new_rig(
        &self,
        root: &std::path::Path,
        name: &str,
    ) -> std::path::PathBuf {
        let source = equip_source(root, name);
        let env = self.equip("new", &source.to_string_lossy());
        assert!(!has_error(&env), "equip new for {name} failed: {env}");
        source.join("rig")
    }

    pub fn scaffold(&self, namepaths: &[&str]) -> json::Value {
        let p = NewParams {
            namepaths: namepaths.iter().map(|s| s.to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.equip_scaffold(nonce, p)))
    }

    /// `interface help` on a namepath or a pattern.
    pub fn inspect(&self, namepath: &str) -> json::Value {
        let p = InspectParams {
            namepath: namepath.to_string(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.help(nonce, p)))
    }

    /// `interface info`, the current view's.
    pub fn info(&self) -> json::Value {
        let nonce = self.interface_nonce(&"info");
        Self::envelope(self.rt.block_on(self.nush.info(nonce)))
    }

    /// `interface purview list`.
    pub fn purviews(&self) -> json::Value {
        let nonce = self.interface_nonce(&"purview list");
        Self::envelope(self.rt.block_on(self.nush.purview_list(nonce)))
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
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.purview_configure(nonce, p)))
    }

    pub fn purview_extend(
        &self,
        purviews: &[&str],
    ) -> json::Value {
        let p = PurviewExtendParams {
            purviews: purviews.iter().map(|s| s.to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.purview_extend(nonce, p)))
    }

    /// `interface purview set`.
    pub fn purview(
        &self,
        purviews: &[&str],
    ) -> json::Value {
        let p = PurviewParams {
            purviews: purviews.iter().map(|s| (*s).to_string()).collect(),
        };
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.purview_set(nonce, p)))
    }

    /// Where this namespace's purview table lands.
    pub fn purviews_path(&self) -> std::path::PathBuf {
        crate::purviews_path()
    }

    /// `interface process list`.
    pub fn processes(&self) -> json::Value {
        let nonce = self.interface_nonce(&"process list");
        Self::envelope(self.rt.block_on(self.nush.process_list(nonce)))
    }

    /// `interface process kill`; its result is null on success.
    pub fn kill(&self, nonce: &str) -> json::Value {
        let p = KillParams {
            nonce: nonce.to_string(),
        };
        let call_nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.process_kill(call_nonce, p)))
    }

    /// `interface channel ready`; its result is null.
    pub fn channel_verified(&self) -> json::Value {
        let nonce = self.interface_nonce(&"channel ready");
        Self::envelope(self.rt.block_on(self.nush.channel_ready(nonce)))
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
        let nonce = self.interface_nonce(&p);
        Self::envelope(self.rt.block_on(self.nush.channel_config(nonce, p)))
    }

    /// `interface channel close`; closing a closed channel succeeds.
    pub fn channel_close(&self) -> json::Value {
        let nonce = self.interface_nonce(&"channel close");
        Self::envelope(self.rt.block_on(self.nush.channel_close(nonce)))
    }

    /// Each routed tool as `tools/list` carries it.
    pub fn tools(&self) -> Vec<json::Value> {
        self.nush
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| json::to_value(&tool).expect("a tool serializes"))
            .collect()
    }

    /// The server's instructions, as `initialize` carries them.
    pub fn instructions(&self) -> String {
        mcp::ServerHandler::get_info(&self.nush)
            .instructions
            .unwrap_or_default()
    }

    /// The in-process equip git repo dir.
    pub fn equip_root(&self) -> std::path::PathBuf {
        crate::equip_root()
    }

    /// The per-call log dir of an `execute` call's nonce.
    pub fn execute_log_dir(&self, nonce: &str) -> std::path::PathBuf {
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

/// True when the envelope is the failure shape, `{nonce, error}`.
pub fn has_error(env: &json::Value) -> bool {
    env.get("error").is_some()
}

/// The envelope's error rows, none on a success.
fn error_rows(env: &json::Value) -> &[json::Value] {
    env.get("error")
        .and_then(|e| e.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The first error row's `kind`, if any.
pub fn error_kind(env: &json::Value) -> Option<&str> {
    error_rows(env).first()?.get("kind")?.as_str()
}

/// Every error row's `kind`.
pub fn error_kinds(env: &json::Value) -> Vec<String> {
    error_rows(env)
        .iter()
        .filter_map(|row| row.get("kind").and_then(|k| k.as_str()))
        .map(str::to_string)
        .collect()
}

pub fn has_kind(env: &json::Value, kind: &str) -> bool {
    error_kinds(env).iter().any(|k| k == kind)
}

/// Every error row's `message`.
pub fn error_messages(env: &json::Value) -> Vec<String> {
    error_rows(env)
        .iter()
        .filter_map(|row| row.get("message").and_then(|m| m.as_str()))
        .map(str::to_string)
        .collect()
}

/// The whole error object rendered to a string.
pub fn error_text(env: &json::Value) -> String {
    env.get("error").map(|e| e.to_string()).unwrap_or_default()
}

/// One rig's slice of an `interface info` signature block.
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

/// Lint a definition as `execute`'s contract does, parse errors aside, as JSON.
pub fn lint_definition(definition: &str) -> Vec<json::Value> {
    ensure_test_config();
    let engine = ParseEngine::new_full();
    let mut ws = nu::StateWorkingSet::new(engine.engine_state());
    let block = nu::parse(&mut ws, Some(LINT_FILE), definition.as_bytes(), false);
    let start = file_start(&ws, LINT_FILE);
    crate::lint_block(&block, &ws, definition, start)
        .iter()
        .map(|d| json::to_value(d).expect("diagnostic serializes"))
        .collect()
}

/// The name `lint_definition` parses a definition under.
const LINT_FILE: &str = "definition.nu";

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

/// The synthesized execute() source for a definition and its literals.
pub fn build_execute_source(
    definition: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> String {
    crate::build_execute_source(definition, data_literal, timeout_literal)
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
