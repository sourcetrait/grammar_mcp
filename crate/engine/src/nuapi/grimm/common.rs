use crate::*;

/// The file the embedded API appends to, inside this call's nonce log dir.
pub(crate) const DEBUG_FILE: &str = "debug.nuonl";

/// The call's stderr log, inside its nonce log dir.
pub(crate) const STDERR_FILE: &str = "stderr";

/// The call's stdout log, inside its nonce log dir.
pub(crate) const STDOUT_FILE: &str = "stdout";

/// The call a host decl acts for: where its artifacts go, and its origin.
pub(crate) struct NuapiCall {
    log_dir: PathBuf,
    origin: String,
}

impl NuapiCall {
    /// The call whose nonce the stack's `$env.NONCE` names, an error outside one.
    pub(crate) fn current(
        engine_state: &nu::EngineState,
        stack: &nu::Stack,
        span: nu::Span,
    ) -> Result<Self, nu::ShellError> {
        let nonce = stack
            .get_env_var(engine_state, "NONCE")
            .and_then(|value| value.as_str().ok())
            .ok_or_else(|| {
                nu::GenericError::new("no call here", "`$env.NONCE` names no call", span)
            })?;
        let log_dir = call_log_dir(nonce).ok_or_else(|| {
            nu::GenericError::new(
                "no call log here",
                format!("no call's log dir answers to the nonce `{nonce}`"),
                span,
            )
        })?;
        Ok(Self {
            log_dir,
            origin: format!("thread/{nonce}"),
        })
    }

    /// This call's stderr log, opened to append after every other writer.
    pub(crate) fn stderr_log(&self, span: nu::Span) -> Result<fs::File, nu::ShellError> {
        let path = self.log_dir.join(STDERR_FILE);
        fs::OpenOptions::new().append(true).open(&path).map_err(|e| {
            nu::GenericError::new(
                format!("cannot open {}", path.display()),
                e.to_string(),
                span,
            )
            .into()
        })
    }

    /// The host-stamped `from` for anything this eval emits.
    pub(crate) fn origin(&self) -> String {
        self.origin.clone()
    }

    /// Append `data` to `<log_dir>/debug.nuonl` as ONE line of NUON.
    pub(crate) fn append_debug(
        &self,
        data: &nu::Value,
        span: nu::Span,
    ) -> Result<(), nu::ShellError> {
        let rendered = nu::to_nuon(&nu::EngineState::new(), data, nu::ToNuonConfig::default())
            .map_err(|e| {
                nu::GenericError::new("cannot render the value as NUON", e.to_string(), span)
            })?;
        let line = rendered.replace('\n', "\\n").replace('\r', "\\r");
        append_line(&self.log_dir.join(DEBUG_FILE), &line).map_err(|e| {
            nu::GenericError::new(format!("cannot append to {DEBUG_FILE}"), e.to_string(), span)
        })?;
        Ok(())
    }
}

/// Append one already-rendered NUON line, creating the parent dir on demand.
fn append_line(
    path: &Path,
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

/// Enforce the declared `oneof<record, table>` at RUNTIME.
pub(crate) fn require_record_or_table(
    data: &nu::Value,
    span: nu::Span,
) -> Result<(), nu::ShellError> {
    match data {
        nu::Value::Record { .. } => Ok(()),
        nu::Value::List { vals, .. }
            if vals.iter().all(|v| matches!(v, nu::Value::Record { .. })) =>
        {
            Ok(())
        }
        other => Err(nu::GenericError::new(
            "expected a record or a table",
            format!("got {}", other.get_type()),
            span,
        )
        .into()),
    }
}

/// The `data` positional shared by the `grimm *` decls: record or table.
pub(crate) fn data_shape() -> nu::SyntaxShape {
    nu::SyntaxShape::OneOf(vec![
        nu::SyntaxShape::Record(nu::CollectionColumns::from(vec![])),
        nu::SyntaxShape::Table(nu::CollectionColumns::from(vec![])),
    ])
}

/// The nullable `data` positional shared by the `grimm *` decls: record or table.
#[allow(unused)]
pub(crate) fn optional_data_shape() -> nu::SyntaxShape {
    nu::SyntaxShape::OneOf(vec![
        nu::SyntaxShape::Record(nu::CollectionColumns::from(vec![])),
        nu::SyntaxShape::Table(nu::CollectionColumns::from(vec![])),
        nu::SyntaxShape::Nothing,
    ])
}

/// The record every `grimm` send returns: the message id under `event_id`.
pub(crate) fn event_id_record(
    id: impl Into<String>,
    span: nu::Span,
) -> nu::Value {
    nu::Value::record(nu::record! { "event_id" => nu::Value::string(id, span) }, span)
}

/// The output type every `grimm` send declares: `record<event_id: string>`.
pub(crate) fn event_id_type() -> nu::Type {
    nu::Type::Record(nu::CollectionColumns::from(vec![(
        "event_id".to_string(),
        nu::Type::String,
    )]))
}

/// Register the `grimm *` decls into the working set an eval will parse with.
pub(crate) fn register_nuapi(working_set: &mut nu::StateWorkingSet) {
    working_set.add_decl(Box::new(GrimmDbg));
    working_set.add_decl(Box::new(GrimmChannelSend));
    working_set.add_decl(Box::new(GrimmGetConfigAll));
    working_set.add_decl(Box::new(GrimmGetConfig));
    working_set.add_decl(Box::new(GrimmPinConfig));
    working_set.add_decl(Box::new(GrimmRemoteChannelSend));
    working_set.add_decl(Box::new(GrimmRemoteChannelSendWith));
}
