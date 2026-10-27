use crate::*;

/// A body's Channel send, as `grimm channel send` hands it to the host.
pub struct ChannelSend<'a> {
    /// The sending call's origin, `thread/<nonce>`.
    pub from: String,
    pub model: String,
    pub event: nu::Value,
    pub attached: Option<nu::Value>,
    /// Stops the sending eval and its job; the action taken, in words.
    pub stop: &'a dyn Fn() -> String,
}

/// A body's send to a linked peer, as the `grimm remote` sends hand it over.
pub struct RemoteSend {
    pub mcp_nom: String,
    pub model: String,
    pub event: nu::Value,
    /// The files table, `src` beside `dest`; `None` for a send without files.
    pub attached: Option<nu::Value>,
}

/// What an eval asks of its host, answered on the eval's thread.
pub trait EngineHost: Send + Sync {
    /// Emits a body's packet on the Channel; its message id, or why not.
    fn channel_send(
        &self,
        send: ChannelSend<'_>,
    ) -> Result<String, String>;

    /// Sends a body's message, with any files, to a peer; its id, or why not.
    fn remote_send(
        &self,
        send: RemoteSend,
    ) -> Result<String, String>;

    /// The configuration in force, as a record mirroring the TOML.
    fn config_record(&self) -> nu::Value;

    /// Holds a setting at a value for a process's lifetime, or says why not.
    fn pin_config(
        &self,
        key: &str,
        pid: u32,
        value: f64,
    ) -> Result<(), String>;
}

/// The host `init` set, once.
static HOST: OnceLock<Box<dyn EngineHost>> = OnceLock::new();

/// Holds `host` as the engine's host; a second one is dropped.
pub(crate) fn set_host(host: Box<dyn EngineHost>) {
    let _ = HOST.set(host);
}

/// The engine's host, or one refusing every request before `init`.
pub(crate) fn host() -> &'static dyn EngineHost {
    match HOST.get() {
        Some(host) => host.as_ref(),
        None => &NoHost,
    }
}

/// The host of an engine nothing started: it refuses, its config empty.
struct NoHost;

/// What `NoHost` answers every request with.
const NO_HOST: &str = "the engine has no host";

impl EngineHost for NoHost {
    fn channel_send(
        &self,
        _send: ChannelSend<'_>,
    ) -> Result<String, String> {
        Err(NO_HOST.to_string())
    }

    fn remote_send(
        &self,
        _send: RemoteSend,
    ) -> Result<String, String> {
        Err(NO_HOST.to_string())
    }

    fn config_record(&self) -> nu::Value {
        nu::Value::record(nu::Record::new(), nu::Span::unknown())
    }

    fn pin_config(
        &self,
        _key: &str,
        _pid: u32,
        _value: f64,
    ) -> Result<(), String> {
        Err(NO_HOST.to_string())
    }
}
