//! A `nu` definition held to the execute contract before anything runs it.
use crate::*;

/// The one name a definition gives its command.
pub(crate) const EXECUTE: &str = "execute";

/// The name a definition is parsed under, which positions its diagnostics.
const DEFINITION_FILE: &str = "definition.nu";

/// The required parameter's name.
const DATA_PARAM: &str = "data";

/// The optional parameter's name.
const TIMEOUT_PARAM: &str = "timeout";

/// The flag nushell gives every custom command.
const HELP_FLAG: &str = "help";

/// The def flag that puts a definition on the stateful lane.
const ENV_FLAG: &str = "env";

/// The kind every contract violation is reported under.
const KIND_DEFINITION: &str = "definition::invalid";

/// What a definition holding to the contract declares.
#[derive(Debug, Clone)]
pub(crate) struct ExecuteDef {
    pub(crate) lane: Lane,
    pub(crate) data_type: NuTypedef,
    pub(crate) result_type: NuTypedef,
    pub(crate) takes_timeout: bool,
}

/// Hold `definition` to the contract, and its body to the lint when `lint`.
pub(crate) fn analyze_definition(
    parse_engine: &ParseEngine,
    definition: &str,
    lint: bool,
) -> Result<ExecuteDef, Vec<Diagnostic>> {
    let mut ws = nu::StateWorkingSet::new(parse_engine.engine_state());
    let block = nu::parse(&mut ws, Some(DEFINITION_FILE), definition.as_bytes(), false);
    let start = file_start(&ws, DEFINITION_FILE);
    let violation = |span: Option<nu::Span>, message: String| -> Diagnostic {
        let position = match span {
            Some(span) => {
                let (line, col) = span_to_line_col(definition, span.start.saturating_sub(start));
                [line, col]
            }
            None => [0, 0],
        };
        Diagnostic::error(
            KIND_DEFINITION,
            Some(Source {
                path: None,
                position,
            }),
            message,
        )
    };
    if !ws.parse_errors.is_empty() {
        let mut rows: Vec<Diagnostic> = Vec::new();
        for error in &ws.parse_errors {
            let row = violation(Some(error.span()), format!("parse error: {error:?}"));
            if !rows.iter().any(|seen| seen.message == row.message) {
                rows.push(row);
            }
        }
        rows.truncate(LINT_VIOLATION_CAP);
        return Err(rows);
    }
    let elements: Vec<&nu::Expression> = block
        .pipelines
        .iter()
        .flat_map(|pipeline| pipeline.elements.iter().map(|element| &element.expr))
        .collect();
    let [expression] = elements.as_slice() else {
        return Err(vec![violation(
            None,
            format!(
                "a definition is exactly one top-level `def {EXECUTE}`, with nothing beside it; \
                 found {} top-level elements",
                elements.len(),
            ),
        )]);
    };
    let call = match &expression.expr {
        nu::Expr::Call(call) if ws.get_decl(call.decl_id).name() == "def" => call,
        nu::Expr::Call(call) => {
            return Err(vec![violation(
                Some(expression.span),
                format!(
                    "the top level holds `def {EXECUTE}`, not `{}`",
                    ws.get_decl(call.decl_id).name(),
                ),
            )]);
        }
        _ => {
            return Err(vec![violation(
                Some(expression.span),
                format!("the top level holds `def {EXECUTE}` alone"),
            )]);
        }
    };
    let name = match call.positional_iter().next().map(|e| &e.expr) {
        Some(nu::Expr::String(name) | nu::Expr::RawString(name)) => name.clone(),
        _ => String::new(),
    };
    if name != EXECUTE {
        return Err(vec![violation(
            Some(expression.span),
            format!("the definition's command is named `{EXECUTE}`, not `{name}`"),
        )]);
    }
    let mut violations = Vec::new();
    let mut lane = Lane::Stateless;
    for argument in &call.arguments {
        if let nu::Argument::Named((flag, _, _)) = argument {
            if flag.item == ENV_FLAG {
                lane = Lane::Stateful;
            } else {
                violations.push(violation(
                    Some(flag.span),
                    format!("`def --{}` is refused; `--{ENV_FLAG}` alone is allowed", flag.item),
                ));
            }
        }
    }
    let Some(decl_id) = ws.find_decl(EXECUTE.as_bytes()) else {
        return Err(vec![violation(
            Some(expression.span),
            format!("`def {EXECUTE}` registered no command"),
        )]);
    };
    let signature = ws.get_decl(decl_id).signature();
    let at = Some(expression.span);
    let data_type = match signature.required_positional.as_slice() {
        [data] if data.name == DATA_PARAM => match data_typedef(&data.shape) {
            Ok(typedef) => Some(typedef),
            Err(reason) => {
                violations.push(violation(at, format!("`{DATA_PARAM}`: {reason}")));
                None
            }
        },
        _ => {
            violations.push(violation(
                at,
                format!("`{EXECUTE}` takes one required parameter, `{DATA_PARAM}`"),
            ));
            None
        }
    };
    let takes_timeout = match signature.optional_positional.as_slice() {
        [] => false,
        [timeout] if timeout.name == TIMEOUT_PARAM => {
            if timeout.shape != nu::SyntaxShape::Duration {
                violations.push(violation(
                    at,
                    format!("`{TIMEOUT_PARAM}?` is typed `duration`, not `{}`", timeout.shape),
                ));
            }
            if timeout.default_value.is_some() {
                violations.push(violation(
                    at,
                    format!("`{TIMEOUT_PARAM}?` takes no default; the call's timeout fills it"),
                ));
            }
            true
        }
        _ => {
            violations.push(violation(
                at,
                format!(
                    "`{EXECUTE}`'s one optional parameter is `{TIMEOUT_PARAM}?: duration`",
                ),
            ));
            false
        }
    };
    if signature.rest_positional.is_some() {
        violations.push(violation(at, format!("`{EXECUTE}` takes no rest parameter")));
    }
    if signature.named.iter().any(|flag| flag.long != HELP_FLAG) {
        violations.push(violation(at, format!("`{EXECUTE}` takes no flags")));
    }
    let result_type = match signature.input_output_types.as_slice() {
        [(nu::Type::Nothing, output)] => match result_typedef(output) {
            Ok(typedef) => Some(typedef),
            Err(reason) => {
                violations.push(violation(at, format!("the return type: {reason}")));
                None
            }
        },
        _ => {
            violations.push(violation(
                at,
                format!(
                    "`{EXECUTE}` declares one infix signature, its input `nothing`: \
                     `: nothing -> record<...>`",
                ),
            ));
            None
        }
    };
    if !violations.is_empty() {
        violations.truncate(LINT_VIOLATION_CAP);
        return Err(violations);
    }
    if lint {
        let findings = lint_block(&block, &ws, definition, start);
        if !findings.is_empty() {
            return Err(findings);
        }
    }
    match (data_type, result_type) {
        (Some(data_type), Some(result_type)) => Ok(ExecuteDef {
            lane,
            data_type,
            result_type,
            takes_timeout,
        }),
        _ => Err(vec![violation(at, "the signature did not resolve".to_string())]),
    }
}
