//! Every tool's declared output schema admits what the tool returns, its
//! success and the error envelope alike, as a client of the 2026-07-28 era
//! validates a structured result against it.

use serde_json::{Value, json};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// What a client's validator reports for `instance` against `schema`.
fn violations(schema: &Value, instance: &Value) -> Vec<String> {
    let validator = jsonschema::validator_for(schema)
        .unwrap_or_else(|e| panic!("a valid JSON schema: {e}; got {schema}"));
    validator.iter_errors(instance).map(|e| e.to_string()).collect()
}

/// `run`'s success and a lint failure's error envelope each satisfy `run`'s
/// declared schema, and that error envelope satisfies every tool's.
#[tested]
fn each_output_schema_admits_its_result_and_the_error_envelope() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let schemas: Vec<(String, Value)> = host
        .tools()
        .into_iter()
        .filter_map(|tool| {
            let schema = tool.get("outputSchema")?.clone();
            Some((tool["name"].as_str().expect("tool name").to_string(), schema))
        })
        .collect();
    assert!(!schemas.is_empty(), "the tools declare output schemas");

    let ok = host.run(json!({
        "args_schema": {"x": "int"},
        "result_schema": {"out": "int"},
        "args": {"x": 5},
        "body": "{ out: ($args.x + 1) }",
    }));
    let failed = host.run(json!({
        "args_schema": {},
        "result_schema": {"x": "string"},
        "args": {},
        "body": "{ x: '/tmp/x' }",
    }));
    let ok = structured(&ok).clone();
    let failed = structured(&failed).clone();
    assert!(failed.get("error").is_some(), "a lint failure's error envelope; got {failed}");

    let run_schema = &schemas
        .iter()
        .find(|(name, _)| name == "run")
        .expect("run declares an output schema")
        .1;
    assert_eq!(violations(run_schema, &ok), Vec::<String>::new(), "run's success: {ok}");
    let refused: Vec<(String, Vec<String>)> = schemas
        .iter()
        .map(|(name, schema)| (name.clone(), violations(schema, &failed)))
        .filter(|(_, errors)| !errors.is_empty())
        .collect();
    assert!(refused.is_empty(), "schemas refusing the error envelope {failed}: {refused:?}");
}
