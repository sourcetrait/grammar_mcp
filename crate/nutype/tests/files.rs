//! The repository's `.nutype` files, read and written back as they stand.
use std::path::Path;

use nutype::NuTypeFile;

/// The repository's `.nutype` files, from the workspace's root.
const FILES: [&str; 5] = [
    "include/config/Config.nutype",
    "build/type/daemon/feature_matrix.nutype",
    "crate/protocol/schema/Schema.nutype",
    "crate/protocol/tests/vectors/Budget.nutype",
    "crate/protocol/tests/vectors/Vectors.nutype",
];

/// Each file's path, its text, and its model.
fn files() -> Vec<(&'static str, String, NuTypeFile)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    FILES
        .into_iter()
        .map(|file| {
            let text = std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"));
            let model = NuTypeFile::from_text(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
            (file, text, model)
        })
        .collect()
}

#[test]
fn each_repository_nutype_reads_and_writes_back_unchanged() {
    for (file, text, model) in files() {
        assert!(!model.doc.is_empty(), "{file} keeps its documentation");
        assert_eq!(model.to_text(), text, "{file} writes back as it stands");
    }
}

/// Each file's type annotates a definition's parameter, and nushell reads it
/// as nutype does.
#[cfg(feature = "nushell")]
#[test]
fn each_repository_nutype_annotates_a_parameter_as_nushell_reads_it() {
    for (file, _, model) in files() {
        let engine = nu_cmd_lang::create_default_context();
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine);
        let source = format!("def t [x: {}] {{}}", model.ty.to_block());
        nu_parser::parse(&mut working_set, None, source.as_bytes(), false);
        assert!(working_set.parse_errors.is_empty(), "{file}: {:?}", working_set.parse_errors);
        let decl = working_set.find_decl(b"t").expect("the definition");
        let shape = &working_set.get_decl(decl).signature().required_positional[0].shape;
        assert_eq!(nutype::NuType::try_from(shape).expect("a shape nutype holds"), model.ty, "{file}");
    }
}
