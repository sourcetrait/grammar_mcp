use crate::*;

pub(crate) async fn run_oneshot(tool: CliTool) -> process::ExitCode {
    let exit_code = async move {
        install_child_subreaper();
        let rig_locks = match ensure_substrate().await {
            Ok(rig_locks) => rig_locks,
            Err(e) => {
                eprintln!("grammar: the equip substrate: {e}");
                return 1;
            }
        };
        let nonce_gen = Arc::new(datum::NonceGenerator::new());
        let lint_engine = Arc::new(LintEngine::new());
        let server = NuSh::new(nonce_gen, rig_locks, lint_engine);
        let result = match tool {
            CliTool::Info => {
                server
                    .info(mcp::Parameters(InfoParams {
                        purviews: Vec::new(),
                    }))
                    .await
            }
            CliTool::Inspect { namepath } => {
                server
                    .inspect(mcp::Parameters(InspectParams { namepath }))
                    .await
            }
            CliTool::Call {
                namepath,
                args,
                timeout_ms,
            } => {
                server
                    .call(mcp::Parameters(CallParams {
                        namepath,
                        args: nuon_record_arg(args.as_deref()),
                        timeout_ms,
                    }))
                    .await
            }
            CliTool::Nu {
                definition,
                data,
                timeout,
            } => {
                server
                    .nu(mcp::Parameters(NuParams {
                        definition,
                        data,
                        timeout,
                    }))
                    .await
            }
            CliTool::Rerun {
                nonce,
                data,
                timeout,
            } => {
                server
                    .rerun(mcp::Parameters(RerunParams {
                        nonce,
                        data,
                        timeout,
                    }))
                    .await
            }
            CliTool::Processes => {
                server
                    .processes(mcp::Parameters(ProcessesParams {}))
                    .await
            }
            CliTool::Kill { nonce } => server.kill(mcp::Parameters(KillParams { nonce })).await,
            CliTool::New { namepaths } => {
                server
                    .scaffold(mcp::Parameters(NewParams { namepaths }))
                    .await
            }
            CliTool::Commit { namepath } => {
                server
                    .commit(mcp::Parameters(CommitParams { namepath }))
                    .await
            }
            CliTool::Equip { action, source_dir } => {
                server
                    .equip(mcp::Parameters(EquipParams {
                        action: action.as_str().to_string(),
                        source_dir,
                    }))
                    .await
            }
        };
        match result {
            Ok(r) => {
                if let Some(mcp::ContentBlock::Text(text)) = r.content.first() {
                    println!("{}", text.text);
                    return if envelope_failed(&text.text) { 1 } else { 0 };
                }
                match r.structured_content {
                    Some(value) => {
                        let failed = value.get("error").is_some();
                        println!("{value}");
                        if failed { 1 } else { 0 }
                    }
                    None => 0,
                }
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        }
    }
    .await;
    process::ExitCode::from(exit_code as u8)
}

/// Whether an envelope's NUON text is the failure shape, holding `error`.
fn envelope_failed(text: &str) -> bool {
    nu::from_nuon(text, None)
        .ok()
        .and_then(|value| value.as_record().ok().map(|record| record.contains("error")))
        .unwrap_or(true)
}

fn nuon_record_arg(input: Option<&str>) -> mcp::JsonObject {
    let Some(raw) = input else {
        return mcp::JsonObject::new();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return mcp::JsonObject::new();
    }
    let value = match nu::from_nuon(trimmed, None) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("invalid NUON record `{trimmed}`: {e}");
            process::exit(2);
        }
    };
    let json_compat = match nu::JsonValue::from_value(value) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("NUON value does not convert to JSON: {e}");
            process::exit(2);
        }
    };
    let serde_value = match json::to_value(&json_compat) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("NUON value does not serialize: {e}");
            process::exit(2);
        }
    };
    match serde_value {
        json::Value::Object(map) => map,
        other => {
            eprintln!("expected a NUON record (e.g. '{{x: 5}}'), got `{other}`");
            process::exit(2);
        }
    }
}
