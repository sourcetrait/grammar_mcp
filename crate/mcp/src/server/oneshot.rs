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
        let server = NuSh::new(nonce_gen, rig_locks, lint_engine, &DenySet::default());
        let result = match tool {
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
            CliTool::Interface { command } => {
                server
                    .interface(mcp::Parameters(InterfaceParams { command }))
                    .await
            }
        };
        match result {
            Ok(r) => match r.content.first() {
                Some(mcp::ContentBlock::Text(text)) => {
                    println!("{}", text.text);
                    if envelope_failed(&text.text) { 1 } else { 0 }
                }
                _ => {
                    eprintln!("grammar: the result carries no envelope text");
                    1
                }
            },
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
