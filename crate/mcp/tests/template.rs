//! Template synthesis + parse-cleanliness. Integration (not unit): `parses_clean`
//! builds a full-shell `ParseEngine` (plugin-registry read off disk), so it rides
//! the resource caveat. Driven through the `guts` surface.

use serde_json::json;
use sourcetrait_grammar_mcp::guts::{build_call_source, build_execute_source, parses_clean};

const DEFINITION: &str =
    "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }";

#[test]
fn nu_source_is_the_definition_then_execute_on_the_data() {
    let got = build_execute_source(DEFINITION, "{x: 5}", None);
    assert_eq!(got, format!("{DEFINITION}\nexecute {{x: 5}}\n"));
    assert!(parses_clean(&got), "a nu source must parse clean:\n{got}");
}

#[test]
fn nu_source_passes_a_null_for_nothing() {
    let definition = "def execute [data: nothing]: nothing -> record<out: int> { { out: 0 } }";
    let got = build_execute_source(definition, "null", None);
    assert_eq!(got, format!("{definition}\nexecute null\n"));
    assert!(parses_clean(&got), "a void nu source must parse clean:\n{got}");
}

#[test]
fn nu_source_passes_the_timeout_to_a_definition_that_takes_it() {
    let definition = "def execute [data: nothing, timeout?: duration]: nothing -> record<out: int> { { out: 0 } }";
    let got = build_execute_source(definition, "null", Some("30sec"));
    assert_eq!(got, format!("{definition}\nexecute null 30sec\n"));
    assert!(parses_clean(&got), "a nu source with a timeout must parse clean:\n{got}");
}

#[test]
fn nu_source_keeps_a_multi_line_definition_verbatim() {
    // The definition is spliced VERBATIM: re-indenting it would corrupt a
    // definition carrying a multi-line string literal.
    let definition = "def execute [data: record<x: int>]: nothing -> record<out: int> {\n    let y = ($data.x * 2)\n{ out: $y }\n}";
    let got = build_execute_source(definition, "{x: 3}", None);
    assert_eq!(got, format!("{definition}\nexecute {{x: 3}}\n"));
    assert!(parses_clean(&got), "a multi-line nu source must parse clean:\n{got}");
}

#[test]
fn call_source_typed_args() {
    let got = build_call_source("sourcetrait/calc", "math", "double", json!({"x": 6}));
    let expected = "overlay use --prefix equip/sourcetrait/calc/rig/math/double as __call\n__call {x: 6}\n";
    assert_eq!(got, expected);
}

#[test]
fn call_source_nested_module_path() {
    let got = build_call_source("sourcetrait/calc", "math/trig", "sin", json!({"x": 1}));
    let expected = "overlay use --prefix equip/sourcetrait/calc/rig/math/trig/sin as __call\n__call {x: 1}\n";
    assert_eq!(got, expected);
}

#[test]
fn call_source_void_args() {
    let got = build_call_source("acme/util", "net", "ping", json!({}));
    let expected = "overlay use --prefix equip/acme/util/rig/net/ping as __call\n__call null\n";
    assert_eq!(got, expected);
}

#[test]
fn call_source_keyword_named_target_is_aliased() {
    let got = build_call_source("sourcetrait/demo", "almost/rpg", "run", json!({"x": 6}));
    let expected = "overlay use --prefix equip/sourcetrait/demo/rig/almost/rpg/run as __call\n__call {x: 6}\n";
    assert_eq!(got, expected);
    assert!(
        !expected.contains("\nrun "),
        "the drive must never be the target's own name",
    );
}
