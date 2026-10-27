use crate::*;
use indoc::formatdoc;

const CALL_ALIAS: &str = "__call";

/// A value as compact NUON, the literal a source carries.
pub(crate) fn render_nuon(value: &nu::Value) -> Result<String, String> {
    nu::to_nuon(&nu::EngineState::new(), value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

/// A call's arguments as their literal: `null` when empty, else their NUON.
fn args_to_nuon(args: &nu::Value) -> String {
    match args {
        nu::Value::Record { val, .. } if val.is_empty() => "null".to_string(),
        value => render_nuon(value).unwrap_or_else(|_| "{}".to_string()),
    }
}

/// serde_json to nu: the one JSON-to-Value bridge in the crate.
pub fn json_value_to_nu_value(v: &serde_json::Value) -> nu::Value {
    let span = nu::Span::unknown();
    match v {
        serde_json::Value::Null => nu::Value::nothing(span),
        serde_json::Value::Bool(b) => nu::Value::bool(*b, span),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                nu::Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                nu::Value::float(f, span)
            } else {
                nu::Value::string(n.to_string(), span)
            }
        }
        serde_json::Value::String(s) => nu::Value::string(s.clone(), span),
        serde_json::Value::Array(arr) => {
            let items: Vec<nu::Value> = arr.iter().map(json_value_to_nu_value).collect();
            nu::Value::list(items, span)
        }
        serde_json::Value::Object(map) => {
            let mut record = nu::Record::new();
            for (k, v) in map {
                record.insert(k.clone(), json_value_to_nu_value(v));
            }
            nu::Value::record(record, span)
        }
    }
}

/// The source of a call: its target's module overlaid, then driven on its args.
pub fn build_call_source(
    rig: &str,
    module_path: &str,
    name: &str,
    args: &nu::Value,
) -> String {
    let lit = args_to_nuon(args);
    let target = if module_path.is_empty() {
        format!("{rig}/rig/{name}")
    } else {
        format!("{rig}/rig/{module_path}/{name}")
    };
    formatdoc!(
        r#"
        overlay use --prefix equip/{target} as {alias}
        {alias} {lit}
    "#,
        target = target,
        alias = CALL_ALIAS,
        lit = lit,
    )
}

/// The source of an `execute` call: its definition, then `execute` on its data.
pub fn build_execute_source(
    definition: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> String {
    let timeout = timeout_literal
        .map(|literal| format!(" {literal}"))
        .unwrap_or_default();
    format!("{definition}\n{EXECUTE} {data_literal}{timeout}\n")
}
