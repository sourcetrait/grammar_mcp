use crate::*;
use indoc::formatdoc;

const CALL_ALIAS: &str = "__call";

fn args_to_nuon(args: &json::JsonObject) -> String {
    let value = json_object_to_nu_value(args);
    let engine_state = nu::EngineState::new();
    nu::to_nuon(&engine_state, &value, nu::ToNuonConfig::default())
        .unwrap_or_else(|_| "{}".to_string())
}

fn json_object_to_nu_value(map: &json::JsonObject) -> nu::Value {
    let span = nu::Span::unknown();
    let mut record = nu::Record::new();
    for (k, v) in map {
        record.insert(k.clone(), json_value_to_nu_value(v));
    }
    nu::Value::record(record, span)
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

pub fn build_call_source(
    rig: &str,
    module_path: &str,
    name: &str,
    args: &json::JsonObject,
) -> String {
    let lit = if args.is_empty() {
        "null".to_string()
    } else {
        args_to_nuon(args)
    };
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
