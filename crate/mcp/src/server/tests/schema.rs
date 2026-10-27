
use crate::*;
use crate::server::schema::{args_schema_to_nu, result_schema_to_nu};

fn obj(s: &str) -> mcp::JsonObject {
    match serde_json::from_str::<json::Value>(s).unwrap() {
        json::Value::Object(m) => m,
        _ => panic!("not an object"),
    }
}


#[test]
fn args_void() {
    assert_eq!(args_schema_to_nu(&obj("{}")).unwrap(), "nothing");
}

#[test]
fn args_simple_record() {
    assert_eq!(
        args_schema_to_nu(&obj(r#"{"a":"int","b":"string"}"#)).unwrap(),
        "record<a: int, b: string>"
    );
}

#[test]
fn args_nested_record_list_table_oneof() {
    let got = args_schema_to_nu(&obj(
        r#"{"r":{"a":"string","b":["int"]},"t":[{"c":"int"}],"u":{"oneof<>":["int",null]}}"#,
    ))
    .unwrap();
    assert_eq!(
        got,
        "record<r: record<a: string, b: list<int>>, t: table<c: int>, u: oneof<int, nothing>>"
    );
}

#[test]
fn args_list_of_oneof_with_record_member() {
    let got = args_schema_to_nu(&obj(r#"{"xs":[{"oneof<>":[{"a":"int"},"string"]}]}"#)).unwrap();
    assert_eq!(got, "record<xs: list<oneof<record<a: int>, string>>>");
}

#[test]
fn args_all_scalars_incl_cell_path() {
    let got = args_schema_to_nu(&obj(
        r#"{"c":"cell-path","d":"directory","n":"number","p":"path"}"#,
    ))
    .unwrap();
    assert_eq!(
        got,
        "record<c: cell-path, d: directory, n: number, p: path>"
    );
}


#[test]
fn deny_any() {
    assert!(args_schema_to_nu(&obj(r#"{"x":"any"}"#)).is_err());
}

#[test]
fn args_allow_open_record_field() {
    assert_eq!(
        args_schema_to_nu(&obj(r#"{"x":{}}"#)).unwrap(),
        "record<x: record<>>"
    );
    assert!(result_schema_to_nu(&obj(r#"{"x":{}}"#)).is_err());
}

#[test]
fn deny_empty_array() {
    assert!(args_schema_to_nu(&obj(r#"{"x":[]}"#)).is_err());
}

#[test]
fn deny_table_of_empty_record() {
    assert!(args_schema_to_nu(&obj(r#"{"x":[{}]}"#)).is_err());
}

#[test]
fn deny_unknown_scalar() {
    assert!(args_schema_to_nu(&obj(r#"{"x":"bogus"}"#)).is_err());
}

#[test]
fn deny_multi_element_array() {
    assert!(args_schema_to_nu(&obj(r#"{"x":["int","string"]}"#)).is_err());
}

#[test]
fn deny_empty_oneof() {
    assert!(args_schema_to_nu(&obj(r#"{"x":{"oneof<>":[]}}"#)).is_err());
}


#[test]
fn emit_void() {
    assert_eq!(nu_to_args_schema("nothing").unwrap(), obj("{}"));
}

#[test]
fn emit_nested() {
    let got = nu_to_args_schema(
        "record<t: record<y: string, n: list<int>>, u: table<a: int>, v: oneof<int, nothing>>",
    )
    .unwrap();
    assert_eq!(
        got,
        obj(r#"{"t":{"y":"string","n":["int"]},"u":[{"a":"int"}],"v":{"oneof<>":["int",null]}}"#)
    );
}

#[test]
fn emit_rejects_top_level_record_empty() {
    assert!(nu_to_args_schema("record<>").is_err());
}

#[test]
fn emit_rejects_bare_list() {
    assert!(nu_to_args_schema("record<x: list>").is_err());
}


#[test]
fn roundtrip_json_nu_json() {
    for s in [
        "{}",
        r#"{"x":"int"}"#,
        r#"{"t":{"y":"string","n":["int"]}}"#,
        r#"{"xs":[{"oneof<>":[{"a":"int"},"string"]}]}"#,
        r#"{"tab":[{"set":"string","count":"int"}]}"#,
    ] {
        let nu = args_schema_to_nu(&obj(s)).unwrap();
        let back = nu_to_args_schema(&nu).unwrap();
        assert_eq!(back, obj(s), "roundtrip failed for {s}");
    }
}

#[test]
fn result_void_and_record() {
    assert_eq!(result_schema_to_nu(&obj("{}")).unwrap(), "nothing");
    assert_eq!(
        result_schema_to_nu(&obj(r#"{"out":"int"}"#)).unwrap(),
        "record<out: int>"
    );
    assert_eq!(nu_to_result_schema("nothing").unwrap(), obj("{}"));
}


#[test]
fn roundtrip_nu_json_nu() {
    for nu in [
        "nothing",
        "record<a: int>",
        "record<a: int, b: string>",
        "record<r: record<x: cell-path, y: list<string>>>",
        "record<t: table<col: int, name: string>>",
        "record<u: oneof<int, nothing>>",
        "record<xs: list<oneof<int, string>>>",
        "record<n: oneof<record<a: int>, string>>",
    ] {
        let json = nu_to_args_schema(nu).unwrap();
        let back = args_schema_to_nu(&json).unwrap();
        assert_eq!(back, nu, "nu roundtrip failed for {nu}");
    }
}


#[test]
fn emit_list_of_record_normalizes_to_table_json() {
    assert_eq!(
        nu_to_args_schema("record<x: list<record<a: int>>>").unwrap(),
        obj(r#"{"x":[{"a":"int"}]}"#)
    );
    assert_eq!(
        nu_to_args_schema("record<x: table<a: int>>").unwrap(),
        obj(r#"{"x":[{"a":"int"}]}"#)
    );
}

#[test]
fn emit_nested_list_and_list_of_table() {
    assert_eq!(
        nu_to_args_schema("record<x: list<list<int>>>").unwrap(),
        obj(r#"{"x":[["int"]]}"#)
    );
    assert_eq!(
        nu_to_args_schema("record<x: list<table<a: int>>>").unwrap(),
        obj(r#"{"x":[[{"a":"int"}]]}"#)
    );
}

#[test]
fn emit_oneof_with_composite_members() {
    assert_eq!(
        nu_to_args_schema("record<u: oneof<record<a: int>, table<b: string>, nothing>>",).unwrap(),
        obj(r#"{"u":{"oneof<>":[{"a":"int"},[{"b":"string"}],null]}}"#)
    );
}


#[test]
fn emit_rejects_bare_table_and_record() {
    assert!(nu_to_args_schema("record<x: table>").is_err());
    assert!(nu_to_args_schema("record<x: record>").is_err());
}

#[test]
fn emit_open_record_field_and_empty_table() {
    assert_eq!(
        nu_to_args_schema("record<x: record<>>").unwrap(),
        obj(r#"{"x":{}}"#)
    );
    assert!(nu_to_args_schema("record<x: table<>>").is_err());
    assert!(nu_to_result_schema("record<x: record<>>").is_err());
}

#[test]
fn args_open_record_oneof_member_and_table_column() {
    assert_eq!(
        args_schema_to_nu(&obj(r#"{"u":{"oneof<>":[{},"string"]}}"#)).unwrap(),
        "record<u: oneof<record<>, string>>"
    );
    assert_eq!(
        nu_to_args_schema("record<u: oneof<record<>, string>>").unwrap(),
        obj(r#"{"u":{"oneof<>":[{},"string"]}}"#)
    );
    assert_eq!(
        args_schema_to_nu(&obj(r#"{"t":[{"x":{}}]}"#)).unwrap(),
        "record<t: table<x: record<>>>"
    );
    assert_eq!(
        nu_to_args_schema("record<t: table<x: record<>>>").unwrap(),
        obj(r#"{"t":[{"x":{}}]}"#)
    );
}

#[test]
fn args_fence_open_record_list_element() {
    assert!(nu_to_args_schema("record<xs: list<record<>>>").is_err());
    assert!(args_schema_to_nu(&obj(r#"{"xs":[{}]}"#)).is_err());
}

#[test]
fn args_open_record_roundtrip() {
    for s in [r#"{"extra":{}}"#, r#"{"a":"int","extra":{}}"#] {
        let nu = args_schema_to_nu(&obj(s)).unwrap();
        assert_eq!(nu_to_args_schema(&nu).unwrap(), obj(s), "roundtrip {s}");
    }
}

#[test]
fn emit_rejects_any_and_unknown() {
    assert!(nu_to_args_schema("record<x: any>").is_err());
    assert!(nu_to_args_schema("record<x: frobnicate>").is_err());
}

#[test]
fn emit_rejects_top_level_bare_forms() {
    assert!(nu_to_args_schema("int").is_err());
    assert!(nu_to_args_schema("list<int>").is_err());
    assert!(nu_to_args_schema("oneof<int, string>").is_err());
}


#[test]
fn deny_oneof_extra_key() {
    assert!(args_schema_to_nu(&obj(r#"{"x":{"oneof<>":["int"],"k":"string"}}"#)).is_err());
}

#[test]
fn deny_oneof_value_not_array() {
    assert!(args_schema_to_nu(&obj(r#"{"x":{"oneof<>":"int"}}"#)).is_err());
}

#[test]
fn deny_bool_and_number_nodes() {
    assert!(args_schema_to_nu(&obj(r#"{"x":true}"#)).is_err());
    assert!(args_schema_to_nu(&obj(r#"{"x":3}"#)).is_err());
}


#[test]
fn all_scalars_roundtrip_both_ways() {
    let names = [
        "int",
        "float",
        "string",
        "bool",
        "datetime",
        "duration",
        "filesize",
        "binary",
        "range",
        "number",
        "glob",
        "cell-path",
        "path",
        "directory",
    ];
    for n in names {
        let json = obj(&format!(r#"{{"f":"{n}"}}"#));
        let nu = format!("record<f: {n}>");
        assert_eq!(args_schema_to_nu(&json).unwrap(), nu, "j2n {n}");
        assert_eq!(nu_to_args_schema(&nu).unwrap(), json, "n2j {n}");
    }
}


#[test]
fn roundtrip_symbols_result_shape() {
    let s = r#"{"suspect_count":"int","suspects":[{"files":["string"],"id":"string"}]}"#;
    let nu = result_schema_to_nu(&obj(s)).unwrap();
    assert_eq!(
        nu,
        "record<suspect_count: int, suspects: table<files: list<string>, id: string>>"
    );
    assert_eq!(nu_to_result_schema(&nu).unwrap(), obj(s));
}

#[test]
fn roundtrip_measure_result_shape() {
    let s = r#"{"failed_picks":"int","per_group":[{"count":"int","group":"string","kp_chars":"int"}],"per_set":[{"count":"int","kp_chars":"int","set":"string"}],"total_kp_chars":"int","valid_picks":"int"}"#;
    let nu = result_schema_to_nu(&obj(s)).unwrap();
    assert_eq!(nu_to_result_schema(&nu).unwrap(), obj(s));
}

#[test]
fn roundtrip_integrity_nested_record_shape() {
    let s = r#"{"counts":{"indexed":"int","memories":"int"},"p1_frontmatter":{"name_mismatch":[{"file":"string","name":"string","slug":"string"}]},"p5_sizes":{"memory_md_bytes":"int","over_cap":"bool"}}"#;
    let nu = result_schema_to_nu(&obj(s)).unwrap();
    assert_eq!(nu_to_result_schema(&nu).unwrap(), obj(s));
}

#[test]
fn args_void_and_validate_result_shape() {
    assert_eq!(args_schema_to_nu(&obj("{}")).unwrap(), "nothing");
    let s = r#"{"failed_details":[{"bytes":"int","pattern":"string","reason":"string"}],"failed_patterns":["string"]}"#;
    let nu = result_schema_to_nu(&obj(s)).unwrap();
    assert_eq!(nu_to_result_schema(&nu).unwrap(), obj(s));
}

fn columns(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::CollectionColumns<nu::SyntaxShape> {
    fields
        .into_iter()
        .map(|(name, shape)| (name.to_string(), shape))
        .collect()
}

fn record(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::SyntaxShape {
    nu::SyntaxShape::Record(columns(fields))
}

fn table(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::SyntaxShape {
    nu::SyntaxShape::Table(columns(fields))
}

fn data(shape: nu::SyntaxShape) -> NuTypedef {
    data_typedef(&shape).expect("a data type")
}

fn checked(shape: nu::SyntaxShape, nuon: &str) -> Result<(), String> {
    let value = nu::from_nuon(nuon, None).expect("NUON");
    crate::server::schema::check_value(&value, &data(shape), "data")
}

#[test]
fn check_holds_a_record_to_its_fields() {
    let shape = || record(vec![("a", nu::SyntaxShape::Int)]);
    assert_eq!(checked(shape(), "{a: 1}"), Ok(()));
    assert_eq!(
        checked(shape(), "{a: 1, b: 2}"),
        Err("data.b: an undeclared field".to_string()),
    );
    assert_eq!(
        checked(shape(), "{}"),
        Err("data.a: a declared field is missing".to_string()),
    );
    assert_eq!(
        checked(shape(), "{a: x}"),
        Err("data.a: expected int, got string".to_string()),
    );
}

#[test]
fn check_takes_anything_inside_an_open_record() {
    let shape = record(vec![("fill", nu::SyntaxShape::record())]);
    assert_eq!(checked(shape, "{fill: {x: 1, y: [2, 3], z: {w: null}}}"), Ok(()));
}

#[test]
fn check_holds_each_table_row_to_its_columns() {
    let shape = || table(vec![("n", nu::SyntaxShape::Int)]);
    assert_eq!(checked(shape(), "[[n]; [1] [2]]"), Ok(()));
    assert_eq!(checked(shape(), "[]"), Ok(()));
    assert_eq!(
        checked(shape(), "[[n, m]; [1, 2]]"),
        Err("data[0].m: an undeclared field".to_string()),
    );
    assert_eq!(
        checked(shape(), "[{n: 1}, {n: 2.5}]"),
        Err("data[1].n: expected int, got float".to_string()),
    );
}

#[test]
fn check_walks_list_items_and_oneof_members() {
    let shape = || {
        record(vec![
            ("xs", nu::SyntaxShape::List(Box::new(nu::SyntaxShape::Int))),
            (
                "u",
                nu::SyntaxShape::OneOf(vec![nu::SyntaxShape::Int, nu::SyntaxShape::Nothing]),
            ),
        ])
    };
    assert_eq!(checked(shape(), "{xs: [1, 2], u: null}"), Ok(()));
    assert_eq!(
        checked(shape(), "{xs: [1, a], u: 1}"),
        Err("data.xs[1]: expected int, got string".to_string()),
    );
    let refused = checked(shape(), "{xs: [], u: s}").expect_err("a string is neither member");
    assert!(refused.starts_with("data.u: a value typed string matches none of"), "{refused}");
}

#[test]
fn check_reads_scalars_by_their_variant() {
    let shape = || {
        record(vec![
            ("p", nu::SyntaxShape::Filepath),
            ("n", nu::SyntaxShape::Number),
            ("d", nu::SyntaxShape::Duration),
        ])
    };
    assert_eq!(checked(shape(), "{p: here, n: 1, d: 2sec}"), Ok(()));
    assert_eq!(checked(shape(), "{p: here, n: 1.5, d: 2sec}"), Ok(()));
    assert_eq!(
        checked(shape(), "{p: here, n: 1, d: 2}"),
        Err("data.d: expected duration, got int".to_string()),
    );
}

#[test]
fn check_holds_nothing_to_null() {
    assert_eq!(checked(nu::SyntaxShape::Nothing, "null"), Ok(()));
    assert_eq!(
        checked(nu::SyntaxShape::Nothing, "{}"),
        Err("data: expected nothing, got record".to_string()),
    );
}
