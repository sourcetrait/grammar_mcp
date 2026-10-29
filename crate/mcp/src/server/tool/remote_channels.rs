use crate::*;

/// Parameters for a remote channel command that names one alias alone.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteAliasParams {
    /// The remote's alias.
    pub alias: String,
}

/// One remote in `interface remote channel list` and `get`: its entry and state.
#[cfg(feature = "embedded-cli")]
pub(crate) type RemoteRow = RemoteChannelRow;

/// A live alias's state: its endpoint, and its peer's nom once paired.
#[cfg(feature = "embedded-cli")]
struct Live {
    endpoint: String,
    remote_mcp_nom: Option<String>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface remote channel list`: every remote, configured or live, by alias.
    pub(crate) async fn remote_list(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.remote_rows() {
            Ok(rows) => success(nonce, &rows),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// `interface remote channel get`: one remote's entry and state.
    pub(crate) async fn remote_get(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.remote_row(&p.alias) {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// One remote's row; an alias neither configured nor live is refused.
    pub(crate) fn remote_row(
        &self,
        alias: &str,
    ) -> Result<RemoteRow, GrammarMcpError> {
        self.remote_rows()?
            .into_iter()
            .find(|row| row.alias == alias)
            .ok_or_else(|| GrammarMcpError::RemoteInvalidParams {
                reason: format!("no remote channel `{alias}`"),
            })
    }

    /// Every remote's row: the file's entries and each live alias, by alias.
    pub(crate) fn remote_rows(&self) -> Result<Vec<RemoteRow>, GrammarMcpError> {
        let entries = remote_entries()?;
        let live = self.live_remotes();
        let mut aliases: Vec<String> = entries
            .iter()
            .map(|entry| entry.alias.clone())
            .chain(live.keys().cloned())
            .collect();
        aliases.sort();
        aliases.dedup();
        Ok(aliases
            .into_iter()
            .map(|alias| {
                let entry = entries.iter().find(|entry| entry.alias == alias);
                let (state, endpoint, remote_mcp_nom) = match live.get(&alias) {
                    Some(Live {
                        endpoint,
                        remote_mcp_nom: Some(nom),
                    }) => (RemoteLinkState::Open, Some(endpoint.clone()), Some(nom.clone())),
                    Some(Live { endpoint, .. }) => {
                        (RemoteLinkState::Listening, Some(endpoint.clone()), None)
                    }
                    None => (RemoteLinkState::Closed, None, None),
                };
                RemoteRow {
                    address: entry.and_then(|entry| entry.address.clone()),
                    listen: entry.and_then(|entry| entry.listen.clone()),
                    cert: entry.map(|entry| entry.cert.clone()),
                    alias,
                    state,
                    endpoint,
                    remote_mcp_nom,
                }
            })
            .collect())
    }

    /// The live aliases: each open link, then each listener still waiting.
    fn live_remotes(&self) -> HashMap<String, Live> {
        let mut live: HashMap<String, Live> = self
            .remote_links
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(alias, link)| {
                let state = Live {
                    endpoint: link.addr.to_string(),
                    remote_mcp_nom: Some(link.handle.remote_mcp_nom.clone()),
                };
                (alias.clone(), state)
            })
            .collect();
        let listeners = bound_listeners();
        let bound = listeners.lock().unwrap_or_else(|e| e.into_inner());
        for (alias, listener) in bound.iter() {
            live.entry(alias.clone()).or_insert(Live {
                endpoint: listener.addr.to_string(),
                remote_mcp_nom: None,
            });
        }
        live
    }

    /// The entry a listen or a connect opens: its Channel verified, the alias idle.
    pub(crate) fn remote_to_open(
        &self,
        alias: &str,
    ) -> Result<RemoteEntry, GrammarMcpError> {
        if !matches!(self.channel.status().phase, ChannelPhase::Verified) {
            return Err(GrammarMcpError::RemoteChannelUnverified);
        }
        if self.live_remotes().contains_key(alias) {
            return Err(GrammarMcpError::RemoteAlreadyOpen {
                alias: alias.to_string(),
            });
        }
        remote_entries()?
            .into_iter()
            .find(|entry| entry.alias == alias)
            .ok_or_else(|| GrammarMcpError::RemoteInvalidParams {
                reason: format!(
                    "no remote channel `{alias}`; add it with `interface remote channel set`"
                ),
            })
    }
}

/// The remotes file's entries; a malformed file is refused whole.
#[cfg(feature = "embedded-cli")]
pub(crate) fn remote_entries() -> Result<Vec<RemoteEntry>, GrammarMcpError> {
    read_remotes(&config().remotes_file).map_err(|reason| GrammarMcpError::RemotesFile { reason })
}

/// The keys a link to `entry`'s peer presents and matches.
#[cfg(feature = "embedded-cli")]
pub(crate) fn link_keys(entry: &RemoteEntry) -> Result<LinkKeys, GrammarMcpError> {
    let identity = config()
        .identity
        .as_ref()
        .ok_or_else(|| GrammarMcpError::RemoteInvalidParams {
            reason: format!(
                "a link presents the host's [identity]; name its `cert` and `key` in \
                 {AGENT_CONFIG_FILE}"
            ),
        })?;
    let peer_cert = entry
        .peer_cert()
        .map_err(|reason| GrammarMcpError::RemoteInvalidParams { reason })?;
    Ok(LinkKeys {
        self_cert: identity.cert.clone(),
        self_key: identity.key.clone(),
        peer_cert,
    })
}
