use crate::*;


#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub(crate) struct RigIndex {
    pub source_path: PathBuf,
    #[serde(default)]
    pub functions: Vec<IndexFunction>,
    #[serde(default)]
    pub modules: Vec<IndexModule>,
}

#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub(crate) struct IndexFunction {
    pub name: String,
    pub args_schema: mcp::JsonObject,
    pub result_schema: mcp::JsonObject,
}

#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub(crate) struct IndexModule {
    pub name: String,
    #[serde(default)]
    pub functions: Vec<IndexFunction>,
    #[serde(default)]
    pub modules: Vec<IndexModule>,
}

/// A lib's index: the source it was committed from.
#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub(crate) struct LibIndex {
    pub source_path: PathBuf,
}

/// The rig's index, under its kind directory.
pub(crate) const META_FILE: &str = ".meta/rig.nuon";

/// The lib's index, under its kind directory.
pub(crate) const LIB_META_FILE: &str = ".meta/lib.nuon";

/// The kind holding an equip's calls.
pub(crate) const KIND_RIG: &str = "rig";

/// The kind holding an equip's plain modules.
pub(crate) const KIND_LIB: &str = "lib";

/// The kinds the MCP installs: a lib first, since its rig may import it.
pub(crate) const EQUIP_KINDS: [&str; 2] = [KIND_LIB, KIND_RIG];

/// The directory that names an equip, in its source path and the namespace.
pub(crate) const EQUIP_DIR: &str = "equip";

/// The environment variable that overrides the deploy's equip directory.
pub(crate) const EQUIP_DIR_ENV: &str = "GRAMMAR_MCP_EQUIP_DIR";


pub(crate) fn keypair_dir() -> PathBuf {
    data_base_dir().join("keypair")
}

pub(crate) fn private_key_path() -> PathBuf {
    keypair_dir().join("id_grammar")
}

pub(crate) fn public_key_path() -> PathBuf {
    keypair_dir().join("id_grammar.pub")
}

pub(crate) fn allowed_signers_path() -> PathBuf {
    keypair_dir().join("allowed_signers")
}

/// The namespace's signed repository of installed equips.
pub(crate) fn equip_root() -> PathBuf {
    data_base_dir().join(EQUIP_DIR)
}

/// An installed equip's directory, its kinds beneath it.
pub(crate) fn equip_dir(equip: &str) -> PathBuf {
    equip_root().join(equip)
}

/// An installed equip's rig.
pub(crate) fn rig_dir(equip: &str) -> PathBuf {
    equip_dir(equip).join(KIND_RIG)
}

/// An installed equip's lib.
pub(crate) fn lib_dir(equip: &str) -> PathBuf {
    equip_dir(equip).join(KIND_LIB)
}

pub(crate) fn is_valid_equip(equip: &str) -> bool {
    match equip.split_once('/') {
        Some((author, name)) => {
            !name.contains('/')
                && is_valid_ident(author)
                && !is_reserved_term(author)
                && is_valid_ident(name)
                && !is_reserved_term(name)
        }
        None => false,
    }
}

pub(crate) fn rig_meta_path(equip: &str) -> PathBuf {
    rig_dir(equip).join(META_FILE)
}

pub(crate) fn lib_meta_path(equip: &str) -> PathBuf {
    lib_dir(equip).join(LIB_META_FILE)
}

pub(crate) fn rig_meta_dir(equip: &str) -> PathBuf {
    rig_dir(equip).join(".meta")
}

pub(crate) fn rig_docs_dir(equip: &str) -> PathBuf {
    rig_meta_dir(equip).join("docs")
}

/// The equip a source path names: its last two components under `equip/`.
pub(crate) fn equip_namepath_of(source_dir: &Path) -> Result<String, GrammarMcpError> {
    let invalid = |reason: &str| GrammarMcpError::EquipInvalidSource {
        source_dir: source_dir.display().to_string(),
        reason: reason.to_string(),
    };
    let mut parts: Vec<&str> = Vec::new();
    for component in source_dir.components() {
        match component {
            std::path::Component::Normal(part) => match part.to_str() {
                Some(s) => parts.push(s),
                None => return Err(invalid("the path is not valid UTF-8")),
            },
            std::path::Component::RootDir => {}
            _ => return Err(invalid("the path must not hold `.` or `..`")),
        }
    }
    let [.., marker, author, name] = parts.as_slice() else {
        return Err(invalid("source_dir must be an `equip/<author>/<name>` directory"));
    };
    if *marker != EQUIP_DIR {
        return Err(invalid("source_dir must be an `equip/<author>/<name>` directory"));
    }
    let equip = format!("{author}/{name}");
    check_equip_name(&equip)?;
    Ok(equip)
}

/// Refuse an equip name that is malformed, reserved, or denied.
fn check_equip_name(equip: &str) -> Result<(), GrammarMcpError> {
    let Some((author, name)) = equip.split_once('/').filter(|(_, n)| !n.contains('/')) else {
        return Err(GrammarMcpError::EquipInvalidName {
            equip: equip.to_string(),
            reason: "an equip is the compound `<author>/<name>`".to_string(),
        });
    };
    for seg in [author, name] {
        if !is_valid_ident(seg) || is_reserved_term(seg) {
            return Err(GrammarMcpError::EquipInvalidName {
                equip: equip.to_string(),
                reason: "author and name must match [a-zA-Z_][a-zA-Z0-9_-]* and not be the reserved `main`"
                    .to_string(),
            });
        }
    }
    if NAME_DENYLIST.contains(&name) {
        return Err(GrammarMcpError::EquipNameDenied {
            equip: equip.to_string(),
        });
    }
    Ok(())
}

/// Refuse an equip that is not installed, or one with no rig.
pub(crate) fn require_rig(equip: &str) -> Result<(), GrammarMcpError> {
    if !equip_dir(equip).exists() {
        return Err(GrammarMcpError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    }
    if !rig_meta_path(equip).exists() {
        return Err(GrammarMcpError::EquipNoRig {
            equip: equip.to_string(),
        });
    }
    Ok(())
}


pub(crate) struct RigLocks {
    map: tk::AsyncMutex<HashMap<String, Arc<tk::AsyncRwLock<()>>>>,
}

impl RigLocks {
    pub(crate) fn new() -> Self {
        Self {
            map: tk::AsyncMutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn hydrate_from_disk(&self) -> io::Result<()> {
        let mut map = self.map.lock().await;
        for equip in registered_equip_names() {
            map.entry(equip)
                .or_insert_with(|| Arc::new(tk::AsyncRwLock::new(())));
        }
        Ok(())
    }

    pub(crate) async fn register(
        &self,
        name: &str,
    ) -> Result<Arc<tk::AsyncRwLock<()>>, AlreadyRegistered> {
        let mut map = self.map.lock().await;
        use std::collections::hash_map::Entry;
        match map.entry(name.to_string()) {
            Entry::Occupied(_) => Err(AlreadyRegistered),
            Entry::Vacant(v) => {
                let lock = Arc::new(tk::AsyncRwLock::new(()));
                v.insert(lock.clone());
                Ok(lock)
            }
        }
    }

    pub(crate) async fn lookup(&self, name: &str) -> Option<Arc<tk::AsyncRwLock<()>>> {
        let map = self.map.lock().await;
        map.get(name).cloned()
    }

    pub(crate) async fn unregister(&self, name: &str) {
        let mut map = self.map.lock().await;
        map.remove(name);
    }
}

pub(crate) struct AlreadyRegistered;


/// Serializes mutations of the ONE equip repo and its namespace meta.
static EQUIP_REPO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn equip_repo_lock() -> std::sync::MutexGuard<'static, ()> {
    EQUIP_REPO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}


const KEY_COMMENT: &str = "grammar@localhost";

pub(crate) fn ensure_keypair() -> io::Result<()> {
    let dir = keypair_dir();
    fs::create_dir_all(&dir)?;
    let priv_path = private_key_path();
    if !priv_path.exists() {
        let status = process::Command::new("ssh-keygen")
            .arg("-q")
            .arg("-t")
            .arg("ed25519")
            .arg("-f")
            .arg(&priv_path)
            .arg("-N")
            .arg("")
            .arg("-C")
            .arg(KEY_COMMENT)
            .status()
            .map_err(|e| io::Error::other(format!("ssh-keygen: {e}")))?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "ssh-keygen failed with status {status}",
            )));
        }
    }
    let pub_key = fs::read_to_string(public_key_path())?;
    let allowed = format!("{} {}", KEY_COMMENT, pub_key.trim());
    fs::write(allowed_signers_path(), allowed.as_bytes())?;
    Ok(())
}

/// Make the namespace's equip repository when absent; true when this call made it.
pub(crate) fn ensure_equip_repo() -> io::Result<bool> {
    let dir = equip_root();
    fs::create_dir_all(&dir)?;
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        configure_repo(&dir)?;
        return Ok(false);
    }
    run_git(&dir, &["init", "-b", "main"])?;
    configure_repo(&dir)?;
    run_git(
        &dir,
        &["commit", "--allow-empty", "-m", "init equip repo"],
    )?;
    Ok(true)
}

fn configure_repo(dir: &std::path::Path) -> io::Result<()> {
    let priv_path = private_key_path();
    let priv_path_str = priv_path
        .to_str()
        .ok_or_else(|| io::Error::other("private key path is not valid UTF-8"))?;
    let allowed_path = allowed_signers_path();
    let allowed_path_str = allowed_path
        .to_str()
        .ok_or_else(|| io::Error::other("allowed_signers path is not valid UTF-8"))?;
    let pairs: [(&str, &str); 6] = [
        ("user.name", "grammar"),
        ("user.email", KEY_COMMENT),
        ("gpg.format", "ssh"),
        ("user.signingKey", priv_path_str),
        ("gpg.ssh.allowedSignersFile", allowed_path_str),
        ("commit.gpgSign", "true"),
    ];
    for (k, v) in pairs {
        run_git(dir, &["config", k, v])?;
    }
    Ok(())
}

pub(crate) fn run_git(dir: &std::path::Path, args: &[&str]) -> io::Result<()> {
    let out = process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| io::Error::other(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr),
        )));
    }
    Ok(())
}


fn skeleton_function_source() -> String {
    "# one-line summary (<= 80 chars); becomes this function's doc\nexport def main [args: record<>]: nothing -> record<> {\n    # logic here; replace each record<> with real fields (or `nothing` for void)\n    {}\n}\n".to_string()
}

fn additively_wire_modnu(modnu: &std::path::Path, entry: &str) -> io::Result<()> {
    let existing = if modnu.exists() {
        fs::read_to_string(modnu)?
    } else {
        String::new()
    };
    if existing.lines().any(|l| l.trim() == entry) {
        return Ok(());
    }
    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(entry);
    out.push('\n');
    fs::write(modnu, out)
}

fn validate_new_names(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<(), GrammarMcpError> {
    check_equip_name(rig)?;
    if !is_valid_module_path(module_path) {
        return Err(GrammarMcpError::RigInvalidName {
            name: module_path.to_string(),
            reason: "invalid module path".to_string(),
        });
    }
    for seg in module_path.split('/').filter(|s| !s.is_empty()) {
        if is_reserved_term(seg) {
            return Err(GrammarMcpError::RigInvalidName {
                name: seg.to_string(),
                reason: "reserved `main` cannot name a module".to_string(),
            });
        }
    }
    if let Some(n) = name {
        if !is_valid_ident(n) || is_reserved_term(n) {
            return Err(GrammarMcpError::RigInvalidName {
                name: n.to_string(),
                reason: "invalid or reserved (`main`) function name".to_string(),
            });
        }
    }
    Ok(())
}

/// Register an empty equip at its source: a rig with an empty root `mod.nu`.
pub(crate) fn establish_equip(
    equip: &str,
    source_dir: &Path,
) -> Result<Vec<String>, GrammarMcpError> {
    check_equip_name(equip)?;
    let _repo = equip_repo_lock();
    if equip_dir(equip).exists() {
        return Err(GrammarMcpError::EquipAlreadyRegistered {
            equip: equip.to_string(),
        });
    }
    let rig_source = source_dir.join(KIND_RIG);
    let canonical = rig_dir(equip);
    fs::create_dir_all(canonical.join(".meta"))?;
    fs::write(canonical.join("mod.nu"), b"")?;
    let index = RigIndex {
        source_path: rig_source.clone(),
        functions: Vec::new(),
        modules: Vec::new(),
    };
    let index_nuon = index_to_nuon(&index).map_err(|reason| GrammarMcpError::Internal {
        phase: "establish::serialize_index".to_string(),
        reason,
    })?;
    fs::write(canonical.join(META_FILE), index_nuon.as_bytes())?;
    run_git(&equip_root(), &["add", "--", equip])?;
    run_git(
        &equip_root(),
        &["commit", "-m", &format!("new equip {equip}")],
    )?;
    fs::create_dir_all(&rig_source)?;
    let root_modnu = rig_source.join("mod.nu");
    if !root_modnu.exists() {
        fs::write(&root_modnu, b"")?;
    }
    Ok(vec![root_modnu.to_string_lossy().into_owned()])
}

pub(crate) fn scaffold_leaf_exists(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<bool, GrammarMcpError> {
    require_rig(rig)?;
    let index = load_index(rig)?;
    let mut dir = index.source_path.clone();
    for seg in module_path.split('/').filter(|s| !s.is_empty()) {
        dir = dir.join(seg);
    }
    let leaf = match name {
        Some(fn_name) => dir.join(fn_name),
        None => dir,
    };
    Ok(leaf.exists())
}

pub(crate) fn scaffold_leaf(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<Vec<String>, GrammarMcpError> {
    validate_new_names(rig, module_path, name)?;
    require_rig(rig)?;
    let index = load_index(rig)?;
    let sp = index.source_path.clone();
    let mut created: Vec<String> = Vec::new();

    let mut dir = sp.clone();
    if !module_path.is_empty() {
        let segs: Vec<&str> = module_path.split('/').collect();
        for (i, seg) in segs.iter().enumerate() {
            let parent_modnu = dir.join("mod.nu");
            let child = dir.join(seg);
            let terminal_module = name.is_none() && i == segs.len() - 1;
            if terminal_module && child.exists() {
                return Err(GrammarMcpError::RigInvalidName {
                    name: module_path.to_string(),
                    reason: "module already exists; edit it instead of scaffolding over it"
                        .to_string(),
                });
            }
            fs::create_dir_all(&child)?;
            let child_modnu = child.join("mod.nu");
            if !child_modnu.exists() {
                fs::write(&child_modnu, b"")?;
            }
            additively_wire_modnu(&parent_modnu, &format!("export module {seg}"))?;
            created.push(child.to_string_lossy().into_owned());
            dir = child;
        }
    }

    if let Some(fn_name) = name {
        let fn_dir = dir.join(fn_name);
        if fn_dir.exists() {
            return Err(GrammarMcpError::RigInvalidName {
                name: fn_name.to_string(),
                reason: "function already exists; edit it instead of scaffolding over it"
                    .to_string(),
            });
        }
        fs::create_dir_all(&fn_dir)?;
        let fn_modnu = fn_dir.join("mod.nu");
        fs::write(&fn_modnu, skeleton_function_source())?;
        let parent_modnu = dir.join("mod.nu");
        additively_wire_modnu(&parent_modnu, &format!("export module {fn_name}"))?;
        additively_wire_modnu(&parent_modnu, &format!("export use {fn_name}"))?;
        created.push(fn_modnu.to_string_lossy().into_owned());
    }

    Ok(created)
}


pub(crate) fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s == "mod" {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().expect("non-empty");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn is_valid_module_path(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.starts_with('/') || s.ends_with('/') || s.contains("//") {
        return false;
    }
    s.split('/').all(is_valid_ident)
}


/// Render a kind's index as NUON - the house format for anything we persist.
fn meta_to_nuon<T: ser::Serialize>(meta: &T) -> Result<String, String> {
    let json = json::to_value(meta).map_err(|e| e.to_string())?;
    let value = json_value_to_nu_value(&json);
    nu::to_nuon(&nu::EngineState::new(), &value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

/// Parse a kind's index back out of NUON, the mirror of `meta_to_nuon`.
fn meta_from_nuon<T: ser::DeserializeOwned>(text: &str) -> Result<T, String> {
    let value = nu::from_nuon(text, None).map_err(|e| e.to_string())?;
    let json_compat = nu::JsonValue::from_value(value).map_err(|e| e.to_string())?;
    let json = json::to_value(&json_compat).map_err(|e| e.to_string())?;
    json::from_value(json).map_err(|e| e.to_string())
}

pub(crate) fn index_to_nuon(index: &RigIndex) -> Result<String, String> {
    meta_to_nuon(index)
}

pub(crate) fn index_from_nuon(text: &str) -> Result<RigIndex, String> {
    meta_from_nuon(text)
}

pub(crate) fn load_index(rig: &str) -> io::Result<RigIndex> {
    let text = fs::read_to_string(rig_meta_path(rig))?;
    index_from_nuon(&text)
        .map_err(|e| io::Error::other(format!("decode index for {rig}: {e}")))
}

pub(crate) fn load_lib_index(equip: &str) -> io::Result<LibIndex> {
    let text = fs::read_to_string(lib_meta_path(equip))?;
    meta_from_nuon(&text)
        .map_err(|e| io::Error::other(format!("decode lib index for {equip}: {e}")))
}



/// A one-line summary from `.meta/docs/<docs_path>/summary.md`, FLATTENED.
fn read_summary(
    docs_dir: &std::path::Path,
    docs_path: &str,
) -> String {
    read_doc(docs_dir, docs_path, "summary.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_doc(docs_dir: &std::path::Path, docs_path: &str, file: &str) -> String {
    let dir = if docs_path.is_empty() {
        docs_dir.to_path_buf()
    } else {
        docs_dir.join(docs_path)
    };
    fs::read_to_string(dir.join(file)).unwrap_or_default()
}

/// Every installed equip's compound `<author>/<name>`, sorted.
pub(crate) fn registered_equip_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let Ok(authors) = fs::read_dir(equip_root()) else {
        return names;
    };
    for author_entry in authors.flatten() {
        if !author_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(author) = author_entry.file_name().into_string() else {
            continue;
        };
        if author.starts_with('.') {
            continue;
        }
        let Ok(equips) = fs::read_dir(author_entry.path()) else {
            continue;
        };
        for equip_entry in equips.flatten() {
            if !equip_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(name) = equip_entry.file_name().into_string() else {
                continue;
            };
            let equip = format!("{author}/{name}");
            if rig_meta_path(&equip).exists() || lib_meta_path(&equip).exists() {
                names.push(equip);
            }
        }
    }
    names.sort();
    names
}

/// Every installed equip with a rig, sorted.
pub(crate) fn registered_rig_names() -> Vec<String> {
    registered_equip_names()
        .into_iter()
        .filter(|equip| rig_meta_path(equip).exists())
        .collect()
}

/// One space per depth level - the indentation IS the structure.
const SIGNATURE_INDENT: &str = " ";

/// `<token> # <summary>`, with the ` # ...` omitted entirely when undocumented.
pub(crate) fn with_summary(
    token: &str,
    summary: &str,
) -> String {
    let summary = summary.trim();
    if summary.is_empty() {
        token.to_string()
    } else {
        format!("{token} # {summary}")
    }
}

/// Append one indented signature line to the block.
fn push_signature_line(
    out: &mut String,
    depth: usize,
    token: &str,
    summary: &str,
) {
    out.push_str(&SIGNATURE_INDENT.repeat(depth));
    out.push_str(&with_summary(token, summary));
    out.push('\n');
}

/// A call's `<name> <args> <result>`; `name` is a leaf or a full namepath.
pub(crate) fn signature_of(
    name: &str,
    f: &IndexFunction,
) -> String {
    let render = |which: &str, rendered: Result<String, String>| -> String {
        rendered.unwrap_or_else(|e| {
            eprintln!("grammar: {which} signature for `{name}` did not render: {e}");
            "<?>".to_string()
        })
    };
    let args = render("args", args_schema_to_signature(&f.args_schema));
    let result = render("result", result_schema_to_signature(&f.result_schema));
    format!("{name} {args} {result}")
}

/// Join a parent module path with a leaf name, tolerating an empty parent.
fn join_path(
    parent: &str,
    name: &str,
) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Render one module's calls and submodules that `patterns` put in view.
fn push_within_signatures(
    out: &mut String,
    depth: usize,
    functions: &[IndexFunction],
    modules: &[IndexModule],
    docs_dir: &std::path::Path,
    parent: &str,
    rig: &str,
    patterns: &[NamepathStr],
) -> bool {
    let mut showed = false;
    let mut calls: Vec<&IndexFunction> = functions.iter().collect();
    calls.sort_by(|a, b| a.name.cmp(&b.name));
    for f in calls {
        let namepath = NamepathRef::Function {
            rig: rig.to_string(),
            module_path: parent.to_string(),
            name: f.name.clone(),
        };
        if !patterns.iter().any(|p| p.covers(&namepath)) {
            continue;
        }
        let docs_path = join_path(parent, &f.name);
        push_signature_line(
            out,
            depth,
            &signature_of(&f.name, f),
            &read_summary(docs_dir, &docs_path),
        );
        showed = true;
    }
    let mut subs: Vec<&IndexModule> = modules.iter().collect();
    subs.sort_by(|a, b| a.name.cmp(&b.name));
    for m in subs {
        let module_path = join_path(parent, &m.name);
        let mut body = String::new();
        let below = push_within_signatures(
            &mut body,
            depth + 1,
            &m.functions,
            &m.modules,
            docs_dir,
            &module_path,
            rig,
            patterns,
        );
        let namepath = NamepathRef::Module {
            rig: rig.to_string(),
            module_path: module_path.clone(),
        };
        let covered = patterns.iter().any(|p| p.covers(&namepath));
        if !below && !covered {
            continue;
        }
        push_signature_line(out, depth, &m.name, &read_summary(docs_dir, &module_path));
        out.push_str(&body);
        showed = true;
    }
    showed
}

/// The namespace as ONE indented signature block, filtered to a set of patterns.
pub(crate) async fn render_signatures_within(
    locks: &RigLocks,
    patterns: &[NamepathStr],
) -> String {
    let mut out = String::new();
    let names = registered_rig_names();
    for name in names.iter().filter(|name| builtin_name(name).is_some()) {
        if let Some((summary, body)) = rig_signatures(locks, name, 1, patterns).await {
            push_signature_line(&mut out, 0, display_equip(name), &summary);
            out.push_str(&body);
        }
    }
    let mut current_author: Option<String> = None;
    for name in names.iter().filter(|name| builtin_name(name).is_none()) {
        let Some((author, leaf)) = name.split_once('/') else {
            continue;
        };
        let Some((summary, body)) = rig_signatures(locks, name, 2, patterns).await else {
            continue;
        };
        if current_author.as_deref() != Some(author) {
            push_signature_line(&mut out, 0, author, "");
            current_author = Some(author.to_string());
        }
        push_signature_line(&mut out, 1, leaf, &summary);
        out.push_str(&body);
    }
    out
}

/// A rig's summary and its signatures in view from `depth`; None when none are.
async fn rig_signatures(
    locks: &RigLocks,
    name: &str,
    depth: usize,
    patterns: &[NamepathStr],
) -> Option<(String, String)> {
    let lock = locks.lookup(name).await;
    let _guard = match &lock {
        Some(l) => Some(l.read().await),
        None => None,
    };
    let index = match load_index(name) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("grammar: signature block skipped {name}: {e}");
            return None;
        }
    };
    let docs_dir = rig_docs_dir(name);
    let mut body = String::new();
    let showed = push_within_signatures(
        &mut body,
        depth,
        &index.functions,
        &index.modules,
        &docs_dir,
        "",
        name,
        patterns,
    );
    let rig_namepath = NamepathRef::Rig {
        rig: name.to_string(),
    };
    let covered = patterns.iter().any(|p| p.covers(&rig_namepath));
    if !showed && !covered {
        return None;
    }
    Some((read_summary(&docs_dir, ""), body))
}

/// A rig's documentation; `srcdir` is the COMMITTED CANONICAL directory.
#[derive(Debug, ser::Serialize)]
pub struct RigDoc {
    pub srcdir: String,
    pub summary: String,
    pub details: String,
}

/// A module's documentation; `src` is its committed `mod.nu`.
#[derive(Debug, ser::Serialize)]
pub struct ModuleDoc {
    pub src: String,
    pub summary: String,
    pub details: String,
}

/// A call's documentation; `src` is the committed `mod.nu` holding its `main`.
#[derive(Debug, ser::Serialize)]
pub struct CallDoc {
    pub src: String,
    pub signature: String,
    pub details: String,
}

/// What a PATTERN inspect returns: the `info()` block rooted at the pattern.
#[derive(Debug, ser::Serialize)]
pub struct SignaturesDoc {
    pub signatures: String,
}

/// What `inspect()` returns - one exact namepath, or a pattern's block.
#[derive(Debug, ser::Serialize)]
#[serde(untagged)]
pub enum InspectDoc {
    Rig(RigDoc),
    Module(ModuleDoc),
    Call(CallDoc),
    Signatures(SignaturesDoc),
}

pub(crate) fn index_node<'a>(
    index: &'a RigIndex,
    module_path: &str,
) -> Option<(&'a Vec<IndexFunction>, &'a Vec<IndexModule>)> {
    if module_path.is_empty() {
        return Some((&index.functions, &index.modules));
    }
    let mut fns = &index.functions;
    let mut mods = &index.modules;
    for seg in module_path.split('/') {
        let m = mods.iter().find(|m| m.name == seg)?;
        fns = &m.functions;
        mods = &m.modules;
    }
    Some((fns, mods))
}

pub(crate) fn inspect_impl(
    rig: &str,
    module_path: &str,
    name: Option<&str>,
) -> Result<InspectDoc, GrammarMcpError> {
    check_equip_name(rig)?;
    require_rig(rig)?;
    if !is_valid_module_path(module_path) {
        return Err(GrammarMcpError::RigInvalidModulePath {
            module_path: module_path.to_string(),
            reason: "invalid module path".to_string(),
        });
    }
    let index = load_index(rig)?;
    let docs_dir = rig_docs_dir(rig);
    let root = rig_dir(rig);
    match name {
        Some(fn_name) => {
            let (fns, _) = index_node(&index, module_path).ok_or_else(|| {
                GrammarMcpError::RigInvalidModulePath {
                    module_path: module_path.to_string(),
                    reason: "module not found".to_string(),
                }
            })?;
            let f = fns.iter().find(|f| f.name == fn_name).ok_or_else(|| {
                GrammarMcpError::FunctionNotDefined {
                    rig: rig.to_string(),
                    module_path: module_path.to_string(),
                    name: fn_name.to_string(),
                }
            })?;
            let docs_path = format!("{module_path}/{fn_name}");
            let namepath = format!("{}:{module_path}:{fn_name}", display_equip(rig));
            Ok(InspectDoc::Call(CallDoc {
                src: root
                    .join(module_path)
                    .join(fn_name)
                    .join("mod.nu")
                    .display()
                    .to_string(),
                signature: with_summary(
                    &signature_of(&namepath, f),
                    &read_summary(&docs_dir, &docs_path),
                ),
                details: read_doc(&docs_dir, &docs_path, "details.md"),
            }))
        }
        None if module_path.is_empty() => Ok(InspectDoc::Rig(RigDoc {
            srcdir: root.display().to_string(),
            summary: read_summary(&docs_dir, ""),
            details: read_doc(&docs_dir, "", "details.md"),
        })),
        None => {
            if index_node(&index, module_path).is_none() {
                return Err(GrammarMcpError::RigInvalidModulePath {
                    module_path: module_path.to_string(),
                    reason: "module not found".to_string(),
                });
            }
            Ok(InspectDoc::Module(ModuleDoc {
                src: root.join(module_path).join("mod.nu").display().to_string(),
                summary: read_summary(&docs_dir, module_path),
                details: read_doc(&docs_dir, module_path, "details.md"),
            }))
        }
    }
}


fn error_count(diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count()
}

fn cap_diagnostics(diagnostics: Vec<Diagnostic>, cap: usize) -> Vec<Diagnostic> {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut out = Vec::new();
    for d in diagnostics {
        match d.severity {
            Severity::Error => {
                if errors < cap {
                    out.push(d);
                    errors += 1;
                }
            }
            Severity::Warning => {
                if warnings < cap {
                    out.push(d);
                    warnings += 1;
                }
            }
        }
    }
    out
}

fn prefix_diagnostic_paths(result: &mut ValidationResult, rig: &str) {
    for d in &mut result.diagnostics {
        if let Some(src) = &mut d.source {
            if let Some(path) = &src.path {
                src.path = Some(format!("{rig}/{path}"));
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    pub functions: Vec<IndexFunction>,
    pub modules: Vec<IndexModule>,
    pub docs: Vec<DocEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct DocEntry {
    pub docs_path: String,
    pub summary: String,
    pub details: String,
}

impl ValidationResult {
    pub(crate) fn is_empty(&self) -> bool {
        error_count(&self.diagnostics) == 0
    }
}

const SUMMARY_MAX_CHARS: usize = 80;

fn extract_doc(source: &str, marker: Option<&str>) -> (String, String, usize) {
    let lines: Vec<&str> = source.lines().collect();
    let doc_idxs: Vec<usize> = match marker {
        None => {
            let mut idxs = Vec::new();
            for (i, l) in lines.iter().enumerate() {
                if l.trim_start().starts_with('#') {
                    idxs.push(i);
                } else {
                    break;
                }
            }
            idxs
        }
        Some(m) => match lines.iter().position(|l| l.trim_start().starts_with(m)) {
            None => Vec::new(),
            Some(decl) => {
                let mut idxs = Vec::new();
                let mut i = decl;
                while i > 0 {
                    i -= 1;
                    if lines[i].trim_start().starts_with('#') {
                        idxs.push(i);
                    } else {
                        break;
                    }
                }
                idxs.reverse();
                idxs
            }
        },
    };
    if doc_idxs.is_empty() {
        return (String::new(), String::new(), 0);
    }
    let stripped: Vec<String> = doc_idxs
        .iter()
        .map(|&i| {
            let t = lines[i].trim_start();
            let t = t.strip_prefix('#').unwrap_or(t);
            t.strip_prefix(' ').unwrap_or(t).to_string()
        })
        .collect();
    let joined = stripped.join("\n");
    let (summary, details) = match joined.split_once("\n\n") {
        Some((s, d)) => (s.to_string(), d.to_string()),
        None => (joined, String::new()),
    };
    (summary, details, doc_idxs[0] + 1)
}

fn check_summary_length(
    rel: &str,
    source: &str,
    marker: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (summary, _details, line) = extract_doc(source, marker);
    if summary.chars().count() > SUMMARY_MAX_CHARS {
        diagnostics.push(Diagnostic::warning(
            "lint::summary_length",
            Some(Source {
                path: Some(rel.to_string()),
                position: [line, 1],
            }),
            "doc summary line exceeds 80 characters",
        ));
    }
}


const NAME_DENYLIST: &[&str] = &["docs", "tools", "bin", "target"];

const ASSET_EXT_DENYLIST: &[&str] = &[
    "nu", "sh", "bash", "zsh", "fish", "ksh", "py", "rb", "pl", "js", "ts", "lua", "ps1", "bat",
    "cmd", "com", "exe",
];

#[derive(Copy, Clone)]
enum Zone {
    Source,
    Doc,
    Asset,
    Lib,
}

impl Zone {
    fn ext_kind(self) -> &'static str {
        match self {
            Zone::Source => "rig::source_extension_denied",
            Zone::Doc => "rig::doc_extension_denied",
            Zone::Asset => "rig::asset_extension_denied",
            Zone::Lib => "lib::extension_denied",
        }
    }

    fn exec_kind(self) -> &'static str {
        match self {
            Zone::Source => "rig::source_executable_denied",
            Zone::Doc => "rig::doc_executable_denied",
            Zone::Asset => "rig::asset_executable_denied",
            Zone::Lib => "lib::executable_denied",
        }
    }
}

fn is_executable(path: &std::path::Path) -> bool {
    fs::metadata(path)
        .map(|m| std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o111 != 0)
        .unwrap_or(false)
}

fn ext_lower(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

fn is_sanctioned_root_file(name: &str) -> bool {
    matches!(
        name,
        "rig.toml" | "README.md" | "LEGAL.md" | "LICENSE.txt"
    ) || (name.starts_with("LICENSE-") && name.ends_with(".txt"))
}

fn push_ext_denied(
    root: &std::path::Path,
    path: &std::path::Path,
    zone: Zone,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    diagnostics.push(Diagnostic::error(
        zone.ext_kind(),
        Some(Source {
            path: Some(rel),
            position: [0, 0],
        }),
        message.to_string(),
    ));
}

fn check_not_executable(
    root: &std::path::Path,
    path: &std::path::Path,
    zone: Zone,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_executable(path) {
        return;
    }
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    diagnostics.push(Diagnostic::error(
        zone.exec_kind(),
        Some(Source {
            path: Some(rel),
            position: [0, 0],
        }),
        "equip files must not be executable (+x); clear the executable bit",
    ));
}

fn validate_assets_tree(
    root: &std::path::Path,
    dir: &std::path::Path,
    result: &mut ValidationResult,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok(());
        }
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            validate_assets_tree(root, &path, result)?;
        } else if ft.is_file() {
            check_not_executable(root, &path, Zone::Asset, &mut result.diagnostics);
            if let Some(ext) = ext_lower(&path)
                && ASSET_EXT_DENYLIST.contains(&ext.as_str())
            {
                push_ext_denied(
                    root,
                    &path,
                    Zone::Asset,
                    ".assets/ must not contain executable/script file types (.sh, .py, .nu, ...)",
                    &mut result.diagnostics,
                );
            }
        }
    }
    Ok(())
}

fn validate_docs_tree(
    root: &std::path::Path,
    dir: &std::path::Path,
    result: &mut ValidationResult,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok(());
        }
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            validate_docs_tree(root, &path, result)?;
        } else if ft.is_file() {
            check_not_executable(root, &path, Zone::Doc, &mut result.diagnostics);
            let allowed = name == ".gitignore"
                || matches!(ext_lower(&path).as_deref(), Some("md") | Some("txt"));
            if !allowed {
                push_ext_denied(
                    root,
                    &path,
                    Zone::Doc,
                    ".docs/ accepts only .md and .txt files (and .gitignore)",
                    &mut result.diagnostics,
                );
            }
        }
    }
    Ok(())
}

struct SelfView {
    base: PathBuf,
}

static SELF_VIEW_SEQ: AtomicU64 = AtomicU64::new(0);

impl SelfView {
    fn new(equip: &str, source_dir: &Path, kinds: &[&str]) -> Option<Self> {
        if !is_valid_equip(equip) {
            return None;
        }
        let base = std::env::temp_dir().join(format!(
            "grammar_selfview_{}_{}",
            process::id(),
            SELF_VIEW_SEQ.fetch_add(1, Ordering::Relaxed),
        ));
        let view = Self { base };
        for kind in kinds {
            let link = view.base.join(EQUIP_DIR).join(equip).join(kind);
            fs::create_dir_all(link.parent()?).ok()?;
            std::os::unix::fs::symlink(source_dir.join(kind), &link).ok()?;
        }
        Some(view)
    }
}

impl Drop for SelfView {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// Validate an equip's source kinds against one self-view, building the rig's index.
pub(crate) fn validate_equip_source(
    equip: &str,
    source_dir: &Path,
    kinds: &[&str],
    engine: &ParseEngine,
) -> io::Result<ValidationResult> {
    let mut result = ValidationResult::default();
    let self_view = SelfView::new(equip, source_dir, kinds);
    let scoped;
    let walk_engine = match &self_view {
        Some(view) => {
            scoped = engine.with_extra_lib_dir(view.base.clone());
            &scoped
        }
        None => engine,
    };
    for kind in kinds {
        let root = source_dir.join(kind);
        let mut found = ValidationResult::default();
        if *kind == KIND_LIB {
            validate_lib_tree(&root, &root, walk_engine, &mut found)?;
        } else {
            let (modules, functions) = validate_walk(&root, &root, walk_engine, "", &mut found)?;
            found.modules = modules;
            found.functions = functions;
        }
        prefix_diagnostic_paths(&mut found, &format!("{equip}/{kind}"));
        result.diagnostics.append(&mut found.diagnostics);
        if *kind == KIND_RIG {
            result.functions = found.functions;
            result.modules = found.modules;
            result.docs = found.docs;
        }
    }
    result.diagnostics = cap_diagnostics(result.diagnostics, LINT_VIOLATION_CAP);
    Ok(result)
}

/// Walk a lib: `.nu` files alone, none executable, each parsed by the engine.
fn validate_lib_tree(
    root: &Path,
    dir: &Path,
    engine: &ParseEngine,
    result: &mut ValidationResult,
) -> io::Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok(());
        }
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            validate_lib_tree(root, &path, engine, result)?;
        } else if ft.is_file() {
            check_not_executable(root, &path, Zone::Lib, &mut result.diagnostics);
            if ext_lower(&path).as_deref() == Some("nu") {
                validate_lib_file(root, &path, engine, result)?;
            } else {
                push_ext_denied(
                    root,
                    &path,
                    Zone::Lib,
                    "a lib holds .nu files alone",
                    &mut result.diagnostics,
                );
            }
        }
    }
    Ok(())
}

/// Parse one lib file as a module; its import cycle or parse errors are findings.
fn validate_lib_file(
    root: &Path,
    path: &Path,
    engine: &ParseEngine,
    result: &mut ValidationResult,
) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    let fname = path.to_string_lossy().into_owned();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("lib");
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let wrapper_name = format!("__lib_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(&source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let files_before = ws.num_files();
    let _ = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&ws, files_before)
    {
        result.diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel),
                position: [0, 0],
            }),
            cycle,
        ));
        return Ok(());
    }
    let source_start = file_start(&ws, &fname) + prefix_len;
    for err in &ws.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(&source, span_start);
        result.diagnostics.push(Diagnostic::error(
            "lib::parse_error",
            Some(Source {
                path: Some(rel.clone()),
                position: [line, 0],
            }),
            format!("parse error: {err:?}"),
        ));
    }
    Ok(())
}

fn validate_walk(
    root: &std::path::Path,
    dir: &std::path::Path,
    engine: &ParseEngine,
    module_path: &str,
    result: &mut ValidationResult,
) -> io::Result<(Vec<IndexModule>, Vec<IndexFunction>)> {
    let modnu = dir.join("mod.nu");
    let modnu_src = if modnu.exists() {
        fs::read_to_string(&modnu).unwrap_or_default()
    } else {
        String::new()
    };
    if modnu.exists() {
        let (summary, details, _) = extract_doc(&modnu_src, None);
        if !summary.is_empty() || !details.is_empty() {
            result.docs.push(DocEntry {
                docs_path: module_path.to_string(),
                summary,
                details,
            });
        }
        let rel = modnu
            .strip_prefix(root)
            .unwrap_or(&modnu)
            .to_string_lossy()
            .into_owned();
        let fname = modnu.to_string_lossy().into_owned();
        if module_path.is_empty()
            && parse_module_has_main(&modnu, "root", &modnu_src, dir, engine)
        {
            result.diagnostics.push(Diagnostic::error(
                "rig::root_function",
                Some(Source {
                    path: Some(rel.clone()),
                    position: [0, 0],
                }),
                "a call-target cannot live at the rig root; move it into a module",
            ));
        }
        validate_mod_nu_ast(&rel, &fname, "mod", &modnu_src, dir, engine, &mut result.diagnostics);
        scan_reserved_terms(&rel, &fname, "mod", &modnu_src, dir, engine, &mut result.diagnostics);
    }
    let edges = extract_module_edges(&modnu_src, &modnu, dir, engine);

    let is_root = module_path.is_empty();
    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if is_root && name == ".assets" {
                validate_assets_tree(root, &path, result)?;
            } else if is_root && name == ".docs" {
                validate_docs_tree(root, &path, result)?;
            } else if name.starts_with('.') {
            } else {
                dirs.push((name, path));
            }
        } else if ft.is_file() {
            let is_nu = path.extension().map(|e| e == "nu").unwrap_or(false);
            if is_nu {
                check_not_executable(root, &path, Zone::Source, &mut result.diagnostics);
                files.push((name, path));
            } else if name == ".gitignore" {
                check_not_executable(root, &path, Zone::Source, &mut result.diagnostics);
            } else if name.starts_with('.') {
            } else {
                check_not_executable(root, &path, Zone::Source, &mut result.diagnostics);
                if !(is_root && is_sanctioned_root_file(&name)) {
                    push_ext_denied(
                        root,
                        &path,
                        Zone::Source,
                        "only .nu files are allowed in the rig tree (plus root README.md / LEGAL.md / LICENSE.txt / LICENSE-*.txt / rig.toml, .gitignore, and the .assets/ + .docs/ dirs)",
                        &mut result.diagnostics,
                    );
                }
            }
        }
    }
    dirs.sort();
    files.sort();

    let mut functions: Vec<IndexFunction> = Vec::new();
    let mut modules: Vec<IndexModule> = Vec::new();

    for (name, path) in &files {
        if name == "mod.nu" {
            continue;
        }
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok((modules, functions));
        }
        validate_flat_file(root, path, engine, result)?;
        let stem = name.trim_end_matches(".nu");
        if !edges.iter().any(|(_, n)| n == stem) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned();
            result.diagnostics.push(Diagnostic::error(
                "rig::orphan",
                Some(Source {
                    path: Some(rel),
                    position: [0, 0],
                }),
                format!(
                    "`{name}` is not reached through the module graph; `export use`/`use` it from mod.nu or remove it",
                ),
            ));
        }
    }

    for (name, path) in &dirs {
        if error_count(&result.diagnostics) > LINT_VIOLATION_CAP {
            return Ok((modules, functions));
        }
        let child_modnu = path.join("mod.nu");
        if !child_modnu.exists() {
            continue;
        }
        let child_src = fs::read_to_string(&child_modnu).unwrap_or_default();
        let rel = child_modnu
            .strip_prefix(root)
            .unwrap_or(&child_modnu)
            .to_string_lossy()
            .into_owned();
        let fname = child_modnu.to_string_lossy().into_owned();
        let edge_kinds: Vec<EdgeKind> = edges
            .iter()
            .filter(|(_, n)| n == name)
            .map(|(k, _)| *k)
            .collect();
        let child_has_main = parse_module_has_main(&child_modnu, name, &child_src, path, engine);

        if child_has_main {
            if module_path.is_empty() {
                result.diagnostics.push(Diagnostic::error(
                    "rig::root_function",
                    Some(Source {
                        path: Some(rel),
                        position: [0, 0],
                    }),
                    "a call-target cannot live at the rig root; move it into a module",
                ));
                continue;
            }
            check_not_executable(root, &child_modnu, Zone::Source, &mut result.diagnostics);
            if edge_kinds.is_empty() {
                result.diagnostics.push(Diagnostic::error(
                    "rig::orphan",
                    Some(Source {
                        path: Some(rel.clone()),
                        position: [0, 0],
                    }),
                    format!("call `{name}` is not wired into mod.nu; add `export use {name}`"),
                ));
            } else if !edge_kinds.contains(&EdgeKind::ExportUse) {
                result.diagnostics.push(Diagnostic::error(
                    "rig::call_wiring",
                    Some(Source {
                        path: Some(rel.clone()),
                        position: [0, 0],
                    }),
                    format!(
                        "a call must be wired into its parent via `export use {name}`, not `export module`",
                    ),
                ));
            }
            if has_child_module_dir(path) {
                result.diagnostics.push(Diagnostic::error(
                    "rig::call_leaf",
                    Some(Source {
                        path: Some(rel.clone()),
                        position: [0, 0],
                    }),
                    format!(
                        "a call target is an edge module and cannot contain submodules; `{name}` has one",
                    ),
                ));
            }
            let extracted = validate_function_file_ast(
                &rel,
                &fname,
                name,
                &child_src,
                path,
                engine,
                &mut result.diagnostics,
            );
            scan_reserved_terms(&rel, &fname, name, &child_src, path, engine, &mut result.diagnostics);
            if let Some((idx_fn, summary, details)) = extracted {
                let docs_path = format!("{module_path}/{name}");
                if !summary.is_empty() || !details.is_empty() {
                    result.docs.push(DocEntry {
                        docs_path,
                        summary,
                        details,
                    });
                }
                functions.push(idx_fn);
            }
        } else {
            if edge_kinds.is_empty() {
                result.diagnostics.push(Diagnostic::error(
                    "rig::orphan",
                    Some(Source {
                        path: Some(rel),
                        position: [0, 0],
                    }),
                    format!("`{name}` is not wired into mod.nu; add `export module {name}` (or remove it)"),
                ));
            }
            let child_path = if module_path.is_empty() {
                name.clone()
            } else {
                format!("{module_path}/{name}")
            };
            let (sub_mods, sub_fns) = validate_walk(root, path, engine, &child_path, result)?;
            if !sub_fns.is_empty() || !sub_mods.is_empty() {
                modules.push(IndexModule {
                    name: name.clone(),
                    functions: sub_fns,
                    modules: sub_mods,
                });
            }
        }
    }

    Ok((modules, functions))
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum EdgeKind {
    ExportModule,
    ExportUse,
    Use,
}

fn parse_module_has_main(
    modnu: &std::path::Path,
    name: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
) -> bool {
    let fname = modnu.to_string_lossy().into_owned();
    let wrapper_name = format!("__cls_{name}");
    let (wrapped, _) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let _ = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty() {
        return false;
    }
    match ws.find_module(wrapper_name.as_bytes()) {
        Some(id) => ws.get_module(id).main.is_some(),
        None => false,
    }
}

fn extract_module_edges(
    source: &str,
    modnu: &std::path::Path,
    parent: &std::path::Path,
    engine: &ParseEngine,
) -> Vec<(EdgeKind, String)> {
    let mut edges = Vec::new();
    if source.is_empty() {
        return edges;
    }
    let fname = modnu.to_string_lossy().into_owned();
    let wrapper_name = "__edges";
    let (wrapped, _) = wrap_as_module(source, wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let block = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty() {
        return edges;
    }
    let body_id = block
        .pipelines
        .first()
        .and_then(|p| p.elements.first())
        .and_then(|elem| match &elem.expr.expr {
            nu::Expr::Call(call) => call.arguments.iter().find_map(|arg| {
                if let nu::Argument::Positional(e) = arg {
                    if let nu::Expr::Block(id) = &e.expr {
                        return Some(*id);
                    }
                }
                None
            }),
            _ => None,
        });
    let Some(body_id) = body_id else {
        return edges;
    };
    let body = ws.get_block(body_id);
    for pipeline in &body.pipelines {
        for elem in &pipeline.elements {
            if let nu::Expr::Call(call) = &elem.expr.expr {
                let decl = ws.get_decl(call.decl_id);
                let kind = match decl.name() {
                    "export module" => EdgeKind::ExportModule,
                    "export use" => EdgeKind::ExportUse,
                    "use" | "overlay use" => EdgeKind::Use,
                    _ => continue,
                };
                if let Some(nu::Argument::Positional(e)) = call.arguments.first() {
                    if let Some(n) = edge_name_from_expr(&e.expr) {
                        edges.push((kind, n));
                    }
                }
            }
        }
    }
    edges
}

fn edge_name_from_expr(expr: &nu::Expr) -> Option<String> {
    let raw = match expr {
        nu::Expr::String(s)
        | nu::Expr::RawString(s)
        | nu::Expr::GlobPattern(s, _)
        | nu::Expr::Filepath(s, _)
        | nu::Expr::Directory(s, _) => s.clone(),
        _ => return None,
    };
    let stem = std::path::Path::new(&raw)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&raw)
        .to_string();
    if stem.is_empty() { None } else { Some(stem) }
}

fn has_child_module_dir(dir: &std::path::Path) -> bool {
    let Ok(read) = fs::read_dir(dir) else {
        return false;
    };
    for entry in read.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if entry.path().join("mod.nu").exists() {
            return true;
        }
    }
    false
}

fn validate_flat_file(
    root: &std::path::Path,
    path: &std::path::Path,
    engine: &ParseEngine,
    result: &mut ValidationResult,
) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    let fname = path.to_string_lossy().into_owned();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let wrapper_name = format!("__of_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(&source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut ws = nu::StateWorkingSet::new(&engine_state);
    let _ = ws.files.push(PathBuf::from(&fname), nu::Span::unknown());
    let files_before = ws.num_files();
    let _ = nu::parse(&mut ws, Some(&fname), wrapped.as_bytes(), false);
    if !ws.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&ws, files_before)
    {
        result.diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel.clone()),
                position: [0, 0],
            }),
            cycle,
        ));
        return Ok(());
    }
    let source_start = file_start(&ws, &fname) + prefix_len;
    for err in &ws.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(&source, span_start);
        result.diagnostics.push(Diagnostic::error(
            "rig::parse_error",
            Some(Source {
                path: Some(rel.clone()),
                position: [line, 0],
            }),
            format!("parse error: {err:?}"),
        ));
    }
    if ws.parse_errors.is_empty()
        && let Some(id) = ws.find_module(wrapper_name.as_bytes())
        && ws.get_module(id).main.is_some()
    {
        result.diagnostics.push(Diagnostic::error(
            "rig::main_in_flat_file",
            Some(Source {
                path: Some(rel.clone()),
                position: [0, 0],
            }),
            "`export def main` is reserved for a call target and must live in `<call>/mod.nu`, not a flat file",
        ));
    }
    scan_reserved_terms(&rel, &fname, stem, &source, parent, engine, &mut result.diagnostics);
    Ok(())
}

fn scan_reserved_terms(
    rel: &str,
    fname: &str,
    stem: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for comp in rel.split('/') {
        let bare = comp.strip_suffix(".nu").unwrap_or(comp);
        if is_reserved_term(bare) {
            diagnostics.push(Diagnostic::error(
                "rig::reserved",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [0, 0],
                }),
                format!(
                    "`{bare}` is reserved (the call-target sentinel) and cannot name a module, directory, or file",
                ),
            ));
        }
    }

    let wrapper_name = format!("__rt_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    let _ = working_set.files.push(PathBuf::from(fname), nu::Span::unknown());
    let block = nu::parse(&mut working_set, Some(fname), wrapped.as_bytes(), false);
    if !working_set.parse_errors.is_empty() {
        return;
    }
    let wrapped_start = file_start(&working_set, fname);
    let source_start = wrapped_start + prefix_len;

    let mut prev_export_def = false;
    for (span, shape) in nu::flatten_block(&working_set, &block) {
        let content = span_text(&wrapped, wrapped_start, span).unwrap_or("");
        let flag = is_reserved_term(content)
            && match &shape {
                nu::FlatShape::VarDecl(_) => true,
                nu::FlatShape::String => !prev_export_def,
                _ => false,
            };
        if flag {
            let src_off = span.start.saturating_sub(source_start);
            let (line, _col) = span_to_line_col(source, src_off);
            diagnostics.push(Diagnostic::error(
                "rig::reserved",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [line, 0],
                }),
                format!(
                    "`{content}` is reserved -- it may appear only as a call-target's exported `main`; rename this identifier",
                ),
            ));
        }
        if matches!(&shape, nu::FlatShape::Signature) {
            for pname in signature_param_names(content) {
                if is_reserved_term(&pname) {
                    let src_off = span.start.saturating_sub(source_start);
                    let (line, _col) = span_to_line_col(source, src_off);
                    diagnostics.push(Diagnostic::error(
                        "rig::reserved",
                        Some(Source {
                            path: Some(rel.to_string()),
                            position: [line, 0],
                        }),
                        format!(
                            "`{pname}` is reserved and cannot be a parameter name; rename it",
                        ),
                    ));
                }
            }
        }
        prev_export_def =
            matches!(&shape, nu::FlatShape::InternalCall(_)) && content == "export def";
    }
}

pub(crate) fn is_reserved_term(s: &str) -> bool {
    s == "main"
}

fn signature_param_names(sig: &str) -> Vec<String> {
    let chars: Vec<char> = sig.chars().collect();
    let n = chars.len();
    let mut names = Vec::new();
    let mut depth: i32 = 0;
    let mut at_slot_start = true;
    let mut i = 0;
    while i < n {
        let c = chars[i];
        match c {
            '[' | '<' | '(' | '{' => {
                if depth > 0 {
                    at_slot_start = false;
                }
                depth += 1;
                i += 1;
            }
            ']' | '>' | ')' | '}' => {
                depth -= 1;
                i += 1;
            }
            ',' if depth <= 1 => {
                at_slot_start = true;
                i += 1;
            }
            ':' => {
                at_slot_start = false;
                i += 1;
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            _ if at_slot_start && depth <= 1 => {
                let start = i;
                while i < n {
                    let cc = chars[i];
                    if cc.is_alphanumeric() || cc == '_' || cc == '-' || cc == '.' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let raw: String = chars[start..i].iter().collect();
                let name = raw
                    .trim_start_matches("...")
                    .trim_start_matches("--")
                    .trim_start_matches('-');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
                at_slot_start = false;
            }
            _ => {
                i += 1;
            }
        }
    }
    names
}

fn validate_mod_nu_ast(
    rel: &str,
    fname: &str,
    stem: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_summary_length(rel, source, None, diagnostics);
    let wrapper_name = format!("__v_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    let _ = working_set.files.push(PathBuf::from(fname), nu::Span::unknown());
    let files_before = working_set.num_files();
    let outer_block = nu::parse(&mut working_set, Some(fname), wrapped.as_bytes(), false);
    let source_start = file_start(&working_set, fname) + prefix_len;

    if !working_set.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&working_set, files_before)
    {
        diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel.to_string()),
                position: [0, 0],
            }),
            cycle,
        ));
        return;
    }

    for err in &working_set.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(source, span_start);
        diagnostics.push(Diagnostic::error(
            "rig::parse_error",
            Some(Source {
                path: Some(rel.to_string()),
                position: [line, 0],
            }),
            format!("parse error: {err:?}"),
        ));
    }

    let body_block_id = outer_block
        .pipelines
        .first()
        .and_then(|p| p.elements.first())
        .and_then(|elem| match &elem.expr.expr {
            nu::Expr::Call(call) => call.arguments.iter().find_map(|arg| {
                if let nu::Argument::Positional(e) = arg {
                    if let nu::Expr::Block(id) = &e.expr {
                        Some(*id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }),
            _ => None,
        });

    let body_block_id: nu::BlockId = match body_block_id {
        Some(id) => id,
        None => {
            return;
        }
    };

    let body = working_set.get_block(body_block_id);
    for pipeline in &body.pipelines {
        for elem in &pipeline.elements {
            check_mod_nu_pipeline_element(
                rel,
                &elem.expr,
                &working_set,
                source,
                source_start,
                diagnostics,
            );
        }
    }
}

fn check_mod_nu_pipeline_element(
    rel: &str,
    expr: &nu::Expression,
    working_set: &nu::StateWorkingSet,
    source: &str,
    source_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span_start = expr.span.start.saturating_sub(source_start);
    let (line, _col) = span_to_line_col(source, span_start);
    match &expr.expr {
        nu::Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);
            let name = decl.name();
            if matches!(
                name,
                "export use"
                    | "export module"
                    | "export const"
                    | "export def"
                    | "use"
                    | "overlay use"
                    | "def"
                    | "const"
                    | "alias"
            ) {
                return;
            }
            diagnostics.push(Diagnostic::error(
                "rig::mod_nu",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [line, 0],
                }),
                format!(
                    "mod.nu may only contain the module cascade (`export module`/`export use`), exports (`export def`/`export const`), or private `def`/`const`/`use`/`alias` declarations; got call to `{name}`",
                ),
            ));
        }
        nu::Expr::Garbage => {
        }
        nu::Expr::ImportPattern(_) | nu::Expr::Overlay(_) => {}
        other => {
            diagnostics.push(Diagnostic::error(
                "rig::mod_nu",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [line, 0],
                }),
                format!(
                    "mod.nu may only contain the module cascade, exports, or private declarations; got `{}`",
                    short_expr_label(other),
                ),
            ));
        }
    }
}

fn short_expr_label(expr: &nu::Expr) -> &'static str {
    match expr {
        nu::Expr::FullCellPath(_) => "cell-path expression",
        nu::Expr::Var(_) => "variable reference",
        nu::Expr::String(_) => "string literal",
        nu::Expr::Int(_) => "integer literal",
        nu::Expr::Float(_) => "float literal",
        nu::Expr::Bool(_) => "bool literal",
        nu::Expr::Block(_) => "block",
        nu::Expr::Closure(_) => "closure",
        nu::Expr::BinaryOp(_, _, _) => "binary operation",
        nu::Expr::Subexpression(_) => "subexpression",
        nu::Expr::Keyword(_) => "keyword",
        _ => "other expression",
    }
}

fn validate_function_file_ast(
    rel: &str,
    fname: &str,
    stem: &str,
    source: &str,
    parent: &std::path::Path,
    engine: &ParseEngine,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(IndexFunction, String, String)> {
    check_summary_length(rel, source, Some("export def main"), diagnostics);
    let wrapper_name = format!("__v_{stem}");
    let (wrapped, prefix_len) = wrap_as_module(source, &wrapper_name);
    let engine_state = engine.engine_state_for_file(parent);
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    let _ = working_set.files.push(PathBuf::from(fname), nu::Span::unknown());
    let files_before = working_set.num_files();
    let _ = nu::parse(&mut working_set, Some(fname), wrapped.as_bytes(), false);
    let wrapped_start = file_start(&working_set, fname);
    let source_start = wrapped_start + prefix_len;

    if !working_set.parse_errors.is_empty()
        && let Some(cycle) = detect_import_cycle(&working_set, files_before)
    {
        diagnostics.push(Diagnostic::error(
            "module::circular_import",
            Some(Source {
                path: Some(rel.to_string()),
                position: [0, 0],
            }),
            cycle,
        ));
        return None;
    }

    for err in &working_set.parse_errors {
        let span_start = err.span().start.saturating_sub(source_start);
        let (line, _col) = span_to_line_col(source, span_start);
        diagnostics.push(Diagnostic::error(
            "rig::parse_error",
            Some(Source {
                path: Some(rel.to_string()),
                position: [line, 0],
            }),
            format!("parse error: {err:?}"),
        ));
    }
    if !working_set.parse_errors.is_empty() {
        return None;
    }

    let wrapper_name_bytes = wrapper_name.as_bytes();
    let module_id = match working_set.find_module(wrapper_name_bytes) {
        Some(id) => id,
        None => {
            diagnostics.push(Diagnostic::error(
                "rig::internal",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [0, 0],
                }),
                "internal: wrapper module not found after parse",
            ));
            return None;
        }
    };
    let module: &nu::Module = working_set.get_module(module_id);

    let main_decl = module.main;
    let Some(main_id) = main_decl else {
        return None;
    };


    check_args_record_positional(rel, &working_set, main_id, "main", source, source_start, diagnostics);
    let main_line = decl_line(&working_set, main_id, source, source_start);
    let out_type = check_main_output_type(
        rel,
        &working_set,
        main_id,
        main_line,
        &wrapped,
        wrapped_start,
        diagnostics,
    );

    let main_sig = working_set.get_decl(main_id).signature();
    let summary = main_sig.description.clone();
    let details = main_sig.extra_description.clone();
    let args_str = extract_main_args_text(&working_set, main_id, &wrapped, wrapped_start)?;
    let result_str = out_type?;
    let args_schema = nu_to_args_schema(&args_str).ok()?;
    let result_schema = nu_to_result_schema(&result_str).ok()?;
    Some((
        IndexFunction {
            name: stem.to_string(),
            args_schema,
            result_schema,
        },
        summary,
        details,
    ))
}

fn check_args_record_positional(
    rel: &str,
    working_set: &nu::StateWorkingSet,
    decl_id: nu::DeclId,
    fn_name: &str,
    source: &str,
    source_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<nu::VarId> {
    let decl = working_set.get_decl(decl_id);
    let sig = decl.signature();
    let positional = sig.required_positional.first();
    let (bad, var_id) = match positional {
        None => (true, None),
        Some(p) => {
            let shape_ok = match p.shape.to_type() {
                nu::Type::Record(fields) => !fields.is_empty(),
                nu::Type::Nothing => true,
                _ => false,
            };
            ((p.name != "args" || !shape_ok), p.var_id)
        }
    };
    if bad {
        let line = decl_line(working_set, decl_id, source, source_start);
        diagnostics.push(Diagnostic::error(
            "rig::args",
            Some(Source {
                path: Some(rel.to_string()),
                position: [line, 0],
            }),
            format!(
                "{fn_name} must take a typed positional `args: record<...>` with real fields (or `args: nothing` for void); an empty `record<>` is the unfleshed skeleton",
            ),
        ));
    }
    var_id
}

fn check_main_output_type(
    rel: &str,
    working_set: &nu::StateWorkingSet,
    decl_id: nu::DeclId,
    line: usize,
    wrapped: &str,
    wrapped_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let sig = working_set.get_decl(decl_id).signature();
    let output = sig
        .input_output_types
        .iter()
        .find(|(input, _)| matches!(input, nu::Type::Nothing))
        .map(|(_, out)| out.clone());
    let Some(output) = output else {
        diagnostics.push(Diagnostic::error(
            "rig::output",
            Some(Source {
                path: Some(rel.to_string()),
                position: [line, 0],
            }),
            "main must declare a `: nothing -> <record<...>|nothing>` output type",
        ));
        return None;
    };
    match &output {
        nu::Type::Nothing => Some("nothing".to_string()),
        nu::Type::Record(fields) if !fields.is_empty() => {
            Some(
                extract_main_output_text(working_set, decl_id, wrapped, wrapped_start)
                    .unwrap_or_else(|| output.to_string()),
            )
        }
        nu::Type::Record(_) => {
            diagnostics.push(Diagnostic::error(
                "rig::skeleton",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [line, 0],
                }),
                "main's output `record<>` is the unfleshed skeleton; give it real fields (or `nothing` for void)",
            ));
            None
        }
        other => {
            diagnostics.push(Diagnostic::error(
                "rig::output",
                Some(Source {
                    path: Some(rel.to_string()),
                    position: [line, 0],
                }),
                format!(
                    "main's output type must be a non-empty `record<...>` or `nothing`; got `{other}`",
                ),
            ));
            None
        }
    }
}

fn extract_main_output_text(
    working_set: &nu::StateWorkingSet,
    decl_id: nu::DeclId,
    wrapped: &str,
    wrapped_start: usize,
) -> Option<String> {
    let block_id = working_set.get_decl(decl_id).block_id()?;
    let body_start = working_set
        .get_block(block_id)
        .span?
        .start
        .checked_sub(wrapped_start)?;
    let before = wrapped.get(..body_start)?;
    let arrow = before.rfind("->")?;
    let after = &before[arrow + 2..];
    let r = after.split('{').next().unwrap_or(after).trim();
    if r.is_empty() {
        None
    } else {
        Some(r.to_string())
    }
}

fn extract_main_args_text(
    working_set: &nu::StateWorkingSet,
    decl_id: nu::DeclId,
    wrapped: &str,
    wrapped_start: usize,
) -> Option<String> {
    let block_id = working_set.get_decl(decl_id).block_id()?;
    let body_start = working_set
        .get_block(block_id)
        .span?
        .start
        .checked_sub(wrapped_start)?;
    let before = wrapped.get(..body_start)?;
    let arrow = before.rfind("->")?;
    let head = &before[..arrow];
    let close = head.rfind(']')?;
    let open = head[..close].rfind('[')?;
    let params = &head[open + 1..close];
    let colon = params.find(':')?;
    let type_text = params[colon + 1..].trim();
    if type_text.is_empty() {
        None
    } else {
        Some(type_text.to_string())
    }
}

fn decl_line(
    working_set: &nu::StateWorkingSet,
    decl_id: nu::DeclId,
    source: &str,
    source_start: usize,
) -> usize {
    let decl = working_set.get_decl(decl_id);
    let span = decl.signature().name.is_empty();
    let _ = span;
    if let Some(block_id) = decl.block_id() {
        let block = working_set.get_block(block_id);
        if let Some(span) = block.span {
            let src_offset = span.start.saturating_sub(source_start);
            let (line, _col) = span_to_line_col(source, src_offset);
            return line;
        }
    }
    0
}


/// A commit's changed paths, grouped by kind, and the validator's warning rows.
#[derive(Debug, ser::Serialize)]
pub(crate) struct CommitResult {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub warnings: Vec<Diagnostic>,
}

impl CommitResult {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}

fn run_git_output(dir: &std::path::Path, args: &[&str]) -> io::Result<String> {
    let out = process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| io::Error::other(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr),
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn parse_git_changes(porcelain: &str) -> CommitResult {
    let mut result = CommitResult {
        added: Vec::new(),
        modified: Vec::new(),
        removed: Vec::new(),
        warnings: Vec::new(),
    };
    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let raw = line[3..].trim();
        let path = raw.rsplit(" -> ").next().unwrap_or(raw).trim().to_string();
        match line.as_bytes()[0] as char {
            'A' | 'C' => result.added.push(path),
            'M' | 'R' => result.modified.push(path),
            'D' => result.removed.push(path),
            _ => {}
        }
    }
    result
}

fn write_rig_meta(
    equip: &str,
    source_path: &Path,
    result: &ValidationResult,
) -> Result<(), GrammarMcpError> {
    let canonical = rig_dir(equip);
    let meta_dir = canonical.join(".meta");
    fs::create_dir_all(&meta_dir)?;
    let index = RigIndex {
        source_path: source_path.to_path_buf(),
        functions: result.functions.clone(),
        modules: result.modules.clone(),
    };
    let index_nuon = index_to_nuon(&index).map_err(|reason| GrammarMcpError::Internal {
        phase: "commit::serialize_index".to_string(),
        reason,
    })?;
    fs::write(canonical.join(META_FILE), index_nuon.as_bytes())?;
    let docs_dir = meta_dir.join("docs");
    for doc in &result.docs {
        let dir = if doc.docs_path.is_empty() {
            docs_dir.clone()
        } else {
            docs_dir.join(&doc.docs_path)
        };
        fs::create_dir_all(&dir)?;
        if !doc.summary.is_empty() {
            fs::write(dir.join("summary.md"), doc.summary.as_bytes())?;
        }
        if !doc.details.is_empty() {
            fs::write(dir.join("details.md"), doc.details.as_bytes())?;
        }
    }
    Ok(())
}

fn write_lib_meta(
    equip: &str,
    source_path: &Path,
) -> Result<(), GrammarMcpError> {
    fs::create_dir_all(lib_dir(equip).join(".meta"))?;
    let index = LibIndex {
        source_path: source_path.to_path_buf(),
    };
    let index_nuon = meta_to_nuon(&index).map_err(|reason| GrammarMcpError::Internal {
        phase: "commit::serialize_lib_index".to_string(),
        reason,
    })?;
    fs::write(lib_meta_path(equip), index_nuon.as_bytes())?;
    Ok(())
}

/// The source an installed equip was committed from: its kinds' parent.
fn registered_source(equip: &str) -> Result<PathBuf, GrammarMcpError> {
    let kind_source = if rig_meta_path(equip).exists() {
        load_index(equip)?.source_path
    } else if lib_meta_path(equip).exists() {
        load_lib_index(equip)?.source_path
    } else {
        return Err(GrammarMcpError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    };
    kind_source
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| GrammarMcpError::Internal {
            phase: "equip::registered_source".to_string(),
            reason: format!("`{}` has no parent", kind_source.display()),
        })
}

/// The kinds an equip's source holds, in install order.
fn source_kinds(source_dir: &Path) -> Vec<&'static str> {
    EQUIP_KINDS
        .iter()
        .copied()
        .filter(|kind| source_dir.join(kind).is_dir())
        .collect()
}

/// The kinds an equip's source holds, refusing a missing source or one with none.
fn require_source_kinds(source_dir: &Path) -> Result<Vec<&'static str>, GrammarMcpError> {
    if !source_dir.is_dir() {
        return Err(GrammarMcpError::EquipSourceMissing {
            path: source_dir.display().to_string(),
        });
    }
    let kinds = source_kinds(source_dir);
    if kinds.is_empty() {
        return Err(GrammarMcpError::EquipNoKinds {
            source_dir: source_dir.display().to_string(),
        });
    }
    Ok(kinds)
}

/// Validate an equip's source, then commit its kinds in place of the last ones.
fn promote_equip(
    equip: &str,
    source_dir: &Path,
    engine: &ParseEngine,
    verb: &str,
) -> Result<CommitResult, GrammarMcpError> {
    let kinds = require_source_kinds(source_dir)?;
    let result = validate_equip_source(equip, source_dir, &kinds, engine)?;
    if !result.is_empty() {
        return Err(GrammarMcpError::EquipViolations {
            diagnostics: result.diagnostics,
        });
    }
    let _repo = equip_repo_lock();
    let canonical = equip_dir(equip);
    if canonical.exists() {
        fs::remove_dir_all(&canonical)?;
    }
    for kind in &kinds {
        let kind_source = source_dir.join(kind);
        if *kind == KIND_LIB {
            copy_tree_all(&kind_source, &lib_dir(equip))?;
            write_lib_meta(equip, &kind_source)?;
        } else {
            copy_dir_recursive(&kind_source, &rig_dir(equip))?;
            write_rig_meta(equip, &kind_source, &result)?;
        }
    }
    run_git(&equip_root(), &["add", "--", equip])?;
    let porcelain = run_git_output(&equip_root(), &["status", "--porcelain", "--", equip])?;
    let mut changed = parse_git_changes(&porcelain);
    changed.warnings = Diagnostic::bucket(result.diagnostics).1;
    if changed.is_empty() {
        return Ok(changed);
    }
    run_git(
        &equip_root(),
        &["commit", "-m", &format!("{verb} equip {equip}")],
    )?;
    Ok(changed)
}

/// Re-commit an installed equip from its registered source.
pub(crate) fn commit_equip(equip: &str, engine: &ParseEngine) -> Result<CommitResult, GrammarMcpError> {
    check_equip_name(equip)?;
    if !equip_dir(equip).exists() {
        return Err(GrammarMcpError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    }
    let source_dir = registered_source(equip)?;
    promote_equip(equip, &source_dir, engine, "commit")
}

pub(crate) fn check_source_dir(equip: &str, source_dir: &Path) -> Result<(), GrammarMcpError> {
    let registered = registered_source(equip)?;
    if registered != source_dir {
        return Err(GrammarMcpError::EquipSourcePathMismatch {
            equip: equip.to_string(),
            passed: source_dir.display().to_string(),
            registered: registered.display().to_string(),
        });
    }
    Ok(())
}

/// Install an equip from its source, validated whole before anything is written.
pub(crate) fn install_equip(
    equip: &str,
    source_dir: &Path,
    engine: &ParseEngine,
) -> Result<CommitResult, GrammarMcpError> {
    check_equip_name(equip)?;
    if equip_dir(equip).exists() {
        return Err(GrammarMcpError::EquipAlreadyRegistered {
            equip: equip.to_string(),
        });
    }
    promote_equip(equip, source_dir, engine, "install")
}

pub(crate) fn uninstall_equip(equip: &str) -> Result<(), GrammarMcpError> {
    let _repo = equip_repo_lock();
    let canonical = equip_dir(equip);
    if !canonical.exists() {
        return Ok(());
    }
    fs::remove_dir_all(&canonical)?;
    run_git(&equip_root(), &["add", "--", equip])?;
    run_git(
        &equip_root(),
        &["commit", "-m", &format!("uninstall equip {equip}")],
    )?;
    Ok(())
}

pub(crate) fn check_equip(
    equip: &str,
    engine: &ParseEngine,
) -> Result<ValidationResult, GrammarMcpError> {
    if !equip_dir(equip).exists() {
        return Err(GrammarMcpError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    }
    let source_dir = registered_source(equip)?;
    let kinds = require_source_kinds(&source_dir)?;
    Ok(validate_equip_source(equip, &source_dir, &kinds, engine)?)
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> io::Result<()> {
    copy_rig_tree(src, dst, true)
}

fn copy_rig_tree(src: &std::path::Path, dst: &std::path::Path, is_root: bool) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if is_root && (name == ".assets" || name == ".docs") {
                copy_tree_all(&src_path, &dst_path)?;
            } else if name.starts_with('.') {
                continue;
            } else {
                copy_rig_tree(&src_path, &dst_path, false)?;
            }
        } else if ft.is_file() && (name == ".gitignore" || !name.starts_with('.')) {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn copy_tree_all(src: &std::path::Path, dst: &std::path::Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_tree_all(&src_path, &dst_path)?;
        } else if ft.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}


pub(crate) async fn ensure_substrate() -> io::Result<Arc<RigLocks>> {
    let fresh = {
        let _repo = equip_repo_lock();
        ensure_keypair()?;
        ensure_equip_repo()?
    };
    if let Err(e) = ensure_default_purview() {
        eprintln!("grammar: default purview not materialized: {e:?}");
    }
    ensure_plugin_registry();
    if fresh {
        install_builtins();
    }
    let locks = Arc::new(RigLocks::new());
    locks.hydrate_from_disk().await?;
    Ok(locks)
}

/// The deploy's equip directory: the override, else `<exe dir>/../equip`.
pub(crate) fn bundled_equip_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(EQUIP_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.join(EQUIP_DIR))
}

/// Install each builtin the deploy holds and seed its purviews.
fn install_builtins() {
    let Some(dir) = bundled_equip_dir() else {
        return;
    };
    let engine = LintEngine::new().current();
    let mut installed: Vec<&str> = Vec::new();
    for (name, author) in BUILTINS {
        let source = dir.join(author).join(name);
        if !source.is_dir() {
            continue;
        }
        let equip = format!("{author}/{name}");
        match install_equip(&equip, &source, &engine) {
            Ok(_) => installed.push(name),
            Err(e) => eprintln!(
                "grammar: builtin {equip} not installed from {}: {e:?}",
                source.display(),
            ),
        }
    }
    if installed.is_empty() {
        return;
    }
    let _repo = equip_repo_lock();
    let mut rows = match load_purviews() {
        Ok(rows) => rows.unwrap_or_default(),
        Err(e) => {
            eprintln!("grammar: builtin purviews not seeded: {e:?}");
            return;
        }
    };
    for name in installed {
        seed_builtin(&mut rows, name);
    }
    if let Err(e) = save_purviews(&rows) {
        eprintln!("grammar: builtin purviews not seeded: {e:?}");
    }
}
