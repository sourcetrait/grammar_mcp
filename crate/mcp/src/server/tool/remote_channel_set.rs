use crate::*;

/// Parameters for `interface remote channel set`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct RemoteSetParams {
    /// The remote to make or change.
    pub alias: String,
    /// The `ip:port` a connect dials by default.
    pub address: Option<String>,
    /// The `ip` or `ip:port` a listen binds by default.
    pub listen: Option<String>,
    /// The peer's certificate, PEM.
    pub cert: Option<String>,
}

impl NuSh {
    /// `interface remote channel set`: make or change an entry, answering its row.
    pub(crate) async fn remote_set(
        &self,
        nonce: datum::Nonce,
        p: RemoteSetParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.set_remote(p) {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// `interface remote channel unset`: remove a remote's entry; a live link stays.
    pub(crate) async fn remote_unset(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match unset_remote(&p.alias) {
            Ok(()) => success_void(nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// Check the fields given, write them to the entry, and read its row back.
    fn set_remote(
        &self,
        p: RemoteSetParams,
    ) -> Result<RemoteRow, GrammarMcpError> {
        let invalid = |reason: String| GrammarMcpError::RemoteInvalidParams { reason };
        if p.alias.is_empty() {
            return Err(invalid("a remote channel's alias is never empty".to_string()));
        }
        if let Some(address) = &p.address {
            dial_address(address).map_err(invalid)?;
        }
        if let Some(listen) = &p.listen {
            bind_address(listen).map_err(invalid)?;
        }
        if p.cert.as_deref() == Some("") {
            return Err(invalid("`--cert` names the peer's certificate".to_string()));
        }
        let known = remote_entries()?.iter().any(|entry| entry.alias == p.alias);
        if !known && p.cert.is_none() {
            return Err(invalid(format!(
                "remote channel `{}` is new; give `--cert`, its peer's certificate",
                p.alias,
            )));
        }
        let change = RemoteChange {
            cert: p.cert,
            address: p.address,
            listen: p.listen,
        };
        edit_remotes(&config().remotes_file, |text| set_entry(text, &p.alias, &change))
            .map_err(|reason| GrammarMcpError::RemotesFile { reason })?;
        self.remote_row(&p.alias)
    }
}

/// Remove `alias`'s entry; an alias the file lacks is refused.
fn unset_remote(alias: &str) -> Result<(), GrammarMcpError> {
    if !remote_entries()?.iter().any(|entry| entry.alias == alias) {
        return Err(GrammarMcpError::RemoteInvalidParams {
            reason: format!("no remote channel `{alias}` in the remotes file"),
        });
    }
    edit_remotes(&config().remotes_file, |text| unset_entry(text, alias))
        .map_err(|reason| GrammarMcpError::RemotesFile { reason })
}
