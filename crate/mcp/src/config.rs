//! The runtime configuration, split format-layer from model-layer.
use crate::*;

/// The embedded base every load merges onto.
const DEFAULTS_CONFIG: &str = include_str!("../../../include/config/grammar_mcp.toml");

/// The namespace when `--namespace` is not given.
pub(crate) const DEFAULT_NAMESPACE: &str = "default";

/// The namespace `--test` defaults to.
pub(crate) const TEST_NAMESPACE: &str = "test";

/// Lowest port the channel hub may be pinned to.
const MIN_CHANNEL_PORT: u16 = 1024;

/// The agent's config file, under the directory the host starts in.
pub(crate) const AGENT_CONFIG_FILE: &str = ".grammar/config.toml";

/// The file shape: the embedded base, and the agent's own file over it.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigToml {
    pub channel: Option<ChannelConfigToml>,
    pub supervisor: Option<SupervisorConfigToml>,
    pub identity: Option<IdentityConfigToml>,
    #[serde(default)]
    pub remote: Vec<RemoteToml>,
}

/// The `[identity]` table (file layer): the host's own key pair.
///
/// The Channel presents it, and so does every `[[remote]]` link.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityConfigToml {
    /// The host's certificate, PEM.
    pub cert_file: Option<String>,
    /// The private key paired with `cert_file`, PEM.
    pub key_file: Option<String>,
}

/// One `[[remote]]` entry (file layer): a peer to link with, by its alias.
///
/// Its role is the presence of `listen`: set, this host binds and waits as the
/// listener; unset, it dials `address` as the connector.
#[derive(Debug, Clone, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemoteToml {
    /// The name remote_channel_open addresses this peer by.
    pub alias: Option<String>,
    /// `ip:port` to bind; present = listener, absent = connector.
    pub listen: Option<String>,
    /// A connector's peer `ip:port`, required; a listener's optional bare source IP.
    pub address: Option<String>,
    /// The peer's known public key to match; `remote_` is implied by the entry.
    pub public_key_file: Option<String>,
}

/// The `[supervisor]` table: when a resource level is worth one warning.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SupervisorConfigToml {
    /// Fraction of ALL cores the host's own CPU must reach.
    pub cpu_warn_fraction: Option<f64>,
    /// Fraction of total system RAM the host's own RSS must reach.
    pub ram_warn_fraction: Option<f64>,
    /// Warn once GPU memory FREE falls below this many MiB.
    pub vram_warn_headroom_mib: Option<u64>,
    /// Fraction of a watched filesystem's capacity.
    pub disk_warn_fraction: Option<f64>,
}

/// The `[channel]` table: the wss port and the spam policy.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChannelConfigToml {
    /// Omit for a kernel-assigned port, which is the default.
    pub port: Option<u16>,
    /// Windows are whole seconds here and on `interface channel config`.
    pub spam_warn_window_secs: Option<u64>,
    pub spam_warn_rate: Option<u32>,
    pub spam_error_window_secs: Option<u64>,
    pub spam_error_rate: Option<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub id: String,
    pub namespace: String,
    pub work_dir: PathBuf,
    pub deny: DenySet,
    /// Was this host launched with `--test`?
    pub test: bool,
    pub channel: ChannelConfig,
    pub supervisor: SupervisorConfig,
    /// The host's own key pair, from `[identity]`; none when the file names none.
    pub identity: Option<IdentityConfig>,
    /// Configured remote peers, from the file's `[[remote]]` entries.
    pub remotes: RemotesConfig,
}

/// The host's own key pair (model layer), each path expanded.
#[derive(Debug, Clone)]
pub(crate) struct IdentityConfig {
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
}

/// A resolved remote peer (model layer): its role and its mTLS key files.
#[derive(Debug, Clone)]
pub(crate) struct RemoteConfig {
    pub alias: String,
    pub role: RemoteRole,
    /// This host's own public key it presents, `[identity]`'s certificate.
    pub self_public_key_file: PathBuf,
    /// The private key paired with `self_public_key_file`, `[identity]`'s key.
    pub self_private_key_file: PathBuf,
    /// The peer's known public key, matched byte-for-byte.
    pub peer_public_key_file: PathBuf,
}

/// The resolved RemoteChannel peer set (model layer), by alias.
#[derive(Debug, Clone, Default)]
pub(crate) struct RemotesConfig {
    pub by_alias: HashMap<String, RemoteConfig>,
}

/// A remote entry's role, decided by the presence of `listen`.
#[derive(Debug, Clone)]
pub(crate) enum RemoteRole {
    /// Dial the peer at this address.
    Connector { addr: std::net::SocketAddr },
    /// Bind here and wait; if `allow` is set, require that inbound source IP.
    Listener {
        bind: std::net::SocketAddr,
        allow: Option<std::net::IpAddr>,
    },
}

/// The channel hub's operating parameters.
#[derive(Debug, Clone)]
pub(crate) struct ChannelConfig {
    /// None means kernel-assigned, which is the default.
    pub port: Option<u16>,
    /// The starting spam policy.
    pub spam: SpamThresholds,
}

/// How much one origin may emit before it is warned, then stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpamThresholds {
    pub warn_window: tk::TkDuration,
    pub warn_rate: u32,
    pub error_window: tk::TkDuration,
    pub error_rate: u32,
}

impl SpamThresholds {
    /// The longer of the two windows.
    pub(crate) fn retention(&self) -> tk::TkDuration {
        if self.warn_window > self.error_window {
            self.warn_window
        } else {
            self.error_window
        }
    }
}

/// Merge one `[channel]` table over the embedded base, then the code defaults.
fn merged_channel(
    user: Option<ChannelConfigToml>,
    base: Option<ChannelConfigToml>,
) -> Result<ChannelConfig, String> {
    let user = user.unwrap_or_default();
    let base = base.unwrap_or_default();
    let port = user.port.or(base.port);
    if let Some(port) = port
        && port < MIN_CHANNEL_PORT
    {
        return Err(format!(
            "channel.port must be {MIN_CHANNEL_PORT}-65535 (below that is privileged); \
             omit it for a kernel-assigned port",
        ));
    }
    let spam = SpamThresholds {
        warn_window: secs_field("spam_warn_window_secs", user.spam_warn_window_secs, base.spam_warn_window_secs)?,
        warn_rate: rate_field("spam_warn_rate", user.spam_warn_rate, base.spam_warn_rate)?,
        error_window: secs_field("spam_error_window_secs", user.spam_error_window_secs, base.spam_error_window_secs)?,
        error_rate: rate_field("spam_error_rate", user.spam_error_rate, base.spam_error_rate)?,
    };
    Ok(ChannelConfig { port, spam })
}

/// The `[identity]` table resolved, both its files required and expanded.
fn resolved_identity(file: Option<IdentityConfigToml>) -> Result<Option<IdentityConfig>, String> {
    let Some(file) = file else {
        return Ok(None);
    };
    let missing = |field: &str| format!("[identity] is missing `{field}`");
    let cert_file = file.cert_file.ok_or_else(|| missing("cert_file"))?;
    let key_file = file.key_file.ok_or_else(|| missing("key_file"))?;
    Ok(Some(IdentityConfig {
        cert_file: expand_path(Path::new(&cert_file))?,
        key_file: expand_path(Path::new(&key_file))?,
    }))
}

fn merged_supervisor(
    user: Option<SupervisorConfigToml>,
    base: Option<SupervisorConfigToml>,
) -> Result<SupervisorConfig, String> {
    let user = user.unwrap_or_default();
    let base = base.unwrap_or_default();
    Ok(SupervisorConfig {
        cpu_warn_fraction: fraction_field(
            "cpu_warn_fraction",
            user.cpu_warn_fraction.or(base.cpu_warn_fraction),
        )?,
        ram_warn_fraction: fraction_field(
            "ram_warn_fraction",
            user.ram_warn_fraction.or(base.ram_warn_fraction),
        )?,
        vram_warn_headroom_mib: user
            .vram_warn_headroom_mib
            .or(base.vram_warn_headroom_mib)
            .ok_or_else(|| "the embedded defaults carry no supervisor.vram_warn_headroom_mib".to_string())?,
        disk_warn_fraction: fraction_field(
            "disk_warn_fraction",
            user.disk_warn_fraction.or(base.disk_warn_fraction),
        )?,
    })
}

impl RemoteConfig {
    /// Resolve one `[[remote]]` entry into a peer, its own key pair the identity's.
    ///
    /// Its role is the presence of `listen`; the peer's public key is required.
    fn resolve(
        entry: RemoteToml,
        identity: &IdentityConfig,
    ) -> Result<Self, String> {
        let alias = entry
            .alias
            .ok_or_else(|| "a [[remote]] entry is missing `alias`".to_string())?;
        let peer_public = entry
            .public_key_file
            .ok_or_else(|| format!("[[remote]] `{alias}` is missing `public_key_file`"))?;
        let role = match entry.listen {
            Some(listen) => {
                let bind = listen.parse::<std::net::SocketAddr>().map_err(|e| {
                    format!("[[remote]] `{alias}` listen `{listen}` is not ip:port: {e}")
                })?;
                let allow = match entry.address {
                    Some(addr) => Some(addr.parse::<std::net::IpAddr>().map_err(|e| {
                        format!(
                            "[[remote]] `{alias}` address `{addr}` is not a bare IP (a listener's `address` is a source-IP filter, no port): {e}"
                        )
                    })?),
                    None => None,
                };
                RemoteRole::Listener { bind, allow }
            }
            None => {
                let addr = entry.address.ok_or_else(|| {
                    format!(
                        "[[remote]] `{alias}` has no `listen`, so it is a connector and needs `address` to dial",
                    )
                })?;
                let addr = addr.parse::<std::net::SocketAddr>().map_err(|e| {
                    format!("[[remote]] `{alias}` address `{addr}` is not ip:port: {e}")
                })?;
                RemoteRole::Connector { addr }
            }
        };
        Ok(RemoteConfig {
            alias,
            role,
            self_public_key_file: identity.cert_file.clone(),
            self_private_key_file: identity.key_file.clone(),
            peer_public_key_file: expand_path(Path::new(&peer_public))?,
        })
    }
}

impl RemotesConfig {
    /// Resolve every `[[remote]]` entry by alias, the identity each presents.
    ///
    /// A duplicate alias fails the load, and so does any entry with no identity.
    pub(crate) fn resolve(
        entries: Vec<RemoteToml>,
        identity: Option<&IdentityConfig>,
    ) -> Result<Self, String> {
        let mut by_alias = HashMap::new();
        if entries.is_empty() {
            return Ok(RemotesConfig { by_alias });
        }
        let identity = identity.ok_or_else(|| {
            "a [[remote]] link presents the host's [identity]; name its `cert_file` and \
             `key_file`"
                .to_string()
        })?;
        for entry in entries {
            let resolved = RemoteConfig::resolve(entry, identity)?;
            if by_alias.contains_key(&resolved.alias) {
                return Err(format!("duplicate [[remote]] alias `{}`", resolved.alias));
            }
            by_alias.insert(resolved.alias.clone(), resolved);
        }
        Ok(RemotesConfig { by_alias })
    }
}

/// A supervisor fraction the merged layers carry, held to the operator's range.
fn fraction_field(
    name: &str,
    value: Option<f64>,
) -> Result<f64, String> {
    match value {
        None => Err(format!("the embedded defaults carry no supervisor.{name}")),
        Some(f) => check_fraction(name, f),
    }
}

/// A window, in whole seconds.
fn secs_field(
    name: &str,
    user: Option<u64>,
    base: Option<u64>,
) -> Result<tk::TkDuration, String> {
    match user.or(base) {
        Some(0) | None => Err(format!("channel.{name} must be a positive number of seconds")),
        Some(secs) => Ok(tk::TkDuration::from_secs(secs)),
    }
}

/// A rate.
fn rate_field(
    name: &str,
    user: Option<u32>,
    base: Option<u32>,
) -> Result<u32, String> {
    match user.or(base) {
        Some(0) | None => Err(format!("channel.{name} must be greater than zero")),
        Some(rate) => Ok(rate),
    }
}

impl Config {
    /// Build the runtime config from the file layer plus the arguments.
    pub(crate) fn from_toml(
        user: ConfigToml,
        id: String,
        namespace: String,
        work_dir: PathBuf,
        deny: DenySet,
        test: bool,
    ) -> Result<Self, String> {
        let base: ConfigToml = toml::from_str(DEFAULTS_CONFIG)
            .map_err(|e| format!("the embedded defaults do not parse: {e}"))?;
        let channel = merged_channel(user.channel, base.channel)?;
        let supervisor = merged_supervisor(user.supervisor, base.supervisor)?;
        let identity = resolved_identity(user.identity)?;
        let remotes = RemotesConfig::resolve(user.remote, identity.as_ref())?;
        Ok(Self {
            id,
            namespace,
            work_dir,
            deny,
            test,
            channel,
            supervisor,
            identity,
            remotes,
        })
    }

    /// Read a user file; an absent or malformed `--config` is an error.
    pub(crate) fn read_toml(path: &std::path::Path) -> Result<ConfigToml, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("read config {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("parse config {}: {e}", path.display()))
    }

    /// Read the agent's `.grammar/config.toml` under `dir`; an absent one is none.
    pub(crate) fn read_agent_toml(dir: &std::path::Path) -> Result<ConfigToml, String> {
        let path = dir.join(AGENT_CONFIG_FILE);
        if !path.exists() {
            return Ok(ConfigToml::default());
        }
        Self::read_toml(&path)
    }
}

impl Default for Config {
    fn default() -> Self {
        let id = default_id();
        let work_dir = default_work_dir(&id);
        Self::from_toml(
            ConfigToml::default(),
            id,
            DEFAULT_NAMESPACE.to_string(),
            work_dir,
            DenySet::default(),
            false,
        )
        .expect("embedded defaults parse")
    }
}

/// The invoking user's name, for the zero-config human case.
pub(crate) fn default_id() -> String {
    std::env::var("USER").unwrap_or_else(|_| "default".to_string())
}

pub(crate) fn default_work_dir(id: &str) -> PathBuf {
    base_dirs().home_dir().join("proj").join("equip").join(id)
}

/// Expand `~` and `$VAR` references so config files stay portable.
pub(crate) fn expand_path(raw: &Path) -> Result<PathBuf, String> {
    let lossy = raw.to_string_lossy();
    if !lossy.contains(['$', '~']) {
        return Ok(raw.to_path_buf());
    }
    shellexpand::full_with_context(lossy.as_ref(), home_dir, |var: &str| {
        if let Ok(value) = std::env::var(var) {
            return Ok(Some(value));
        }
        match spec_default(var) {
            Some(default) => Ok(Some(default.to_string_lossy().into_owned())),
            None => Err("environment variable not found".to_string()),
        }
    })
    .map(|expanded| PathBuf::from(expanded.as_ref()))
    .map_err(|e| format!("expand {}: ${}: {}", raw.display(), e.var_name, e.cause))
}

/// `$HOME`, read per call: the tilde source and the spec defaults' base.
fn home_dir() -> Option<String> {
    std::env::var("HOME").ok()
}

/// The XDG base directory spec's default for an unset variable of its own.
pub(crate) fn spec_default(var: &str) -> Option<PathBuf> {
    let home_relative = match var {
        "XDG_CACHE_HOME" => ".cache",
        "XDG_CONFIG_HOME" => ".config",
        "XDG_DATA_HOME" => ".local/share",
        "XDG_STATE_HOME" => ".local/state",
        _ => return None,
    };
    home_dir().map(|home| PathBuf::from(home).join(home_relative))
}

/// A tool `--deny` withholds, or one interface command by its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeniableTool {
    Execute,
    Rerun,
    Call,
    Interface,
    Command(&'static str),
}

impl DeniableTool {
    pub(crate) fn from_name(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "execute" => Self::Execute,
            "rerun" => Self::Rerun,
            "call" => Self::Call,
            "interface" => Self::Interface,
            words => Self::Command(command_by_words(words)?.words()),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DenySet {
    denied: Vec<DeniableTool>,
}

impl DenySet {
    pub(crate) fn new(denied: Vec<DeniableTool>) -> Self {
        Self { denied }
    }

    pub(crate) fn denies(
        &self,
        tool: DeniableTool,
    ) -> bool {
        self.denied.contains(&tool)
    }

    /// Whether the interface command its words name is withheld.
    pub(crate) fn denies_command(
        &self,
        words: &str,
    ) -> bool {
        self.denied
            .iter()
            .any(|tool| matches!(tool, DeniableTool::Command(denied) if *denied == words))
    }
}

pub(crate) static CONFIG: OnceLock<Config> = OnceLock::new();

pub(crate) fn config() -> &'static Config {
    CONFIG.get().expect("CONFIG set at startup")
}
