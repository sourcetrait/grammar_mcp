//! The daemon fixtures' public files: the trust vectors, each case OpenSSH's
//! own outcome under a placeholder name, and the identity manifest's schema,
//! neither holding any private key data.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value;

/// The placeholder every vector names in place of an endpoint.
const ALIAS: &str = "endpoint.invalid";

/// A fixture file's text from `tests/daemon/`.
fn text(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("daemon").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A fixture file parsed as JSON.
fn json(name: &str) -> Value {
    serde_json::from_str(&text(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// A JSON array's members, refused when the value is not one.
fn items<'a>(value: &'a Value, what: &str) -> &'a Vec<Value> {
    value.as_array().unwrap_or_else(|| panic!("{what} is not an array"))
}

#[test]
fn the_trust_vectors_hold_public_material_alone() {
    let raw = text("trust_vectors.json");
    assert!(!raw.contains("PRIVATE KEY"), "a private key in the public vectors");
    let vectors = json("trust_vectors.json");
    assert_eq!(vectors["schema_version"], 1);
    assert_eq!(vectors["alias"], ALIAS);
    let host = vectors["host_key"].as_str().expect("the host key");
    assert!(host.starts_with("ssh-ed25519 "), "{host}");
    for ca in items(&vectors["cas"], "cas") {
        assert!(ca["public"].as_str().expect("a CA's key").starts_with("ssh-ed25519 "));
    }
    for client in items(&vectors["clients"], "clients") {
        let public = client["public"].as_str().expect("a client's key");
        assert!(public.starts_with("ssh-ed25519 ") || public.starts_with("ssh-rsa "), "{public}");
    }
}

#[test]
fn every_vector_case_observed_its_expected_class() {
    let vectors = json("trust_vectors.json");
    let cases = items(&vectors["cases"], "cases");
    let names: BTreeSet<&str> = cases.iter().map(|c| c["case"].as_str().expect("a case")).collect();
    assert_eq!(names.len(), cases.len(), "a case name repeats");
    for case in cases {
        assert_eq!(case["observed"], case["expect"], "{}", case["case"]);
    }
    let classes: BTreeSet<&str> =
        cases.iter().map(|c| c["expect"].as_str().expect("a class")).collect();
    assert_eq!(classes, ["accept", "auth", "forward", "host_trust"].into_iter().collect());
}

#[test]
fn every_known_hosts_line_names_the_placeholder() {
    let vectors = json("trust_vectors.json");
    for file in items(&vectors["known_hosts"], "known_hosts") {
        for line in items(&file["lines"], "lines") {
            let line = line.as_str().expect("a line");
            assert!(line.contains(ALIAS) || line.starts_with("@revoked * "), "{line}");
        }
    }
}

#[test]
fn every_certificate_names_its_case_principal_and_serial() {
    let vectors = json("trust_vectors.json");
    let certs = items(&vectors["certs"], "certs");
    let mut serials = BTreeSet::new();
    for cert in certs {
        let case = cert["case"].as_str().expect("a case");
        let body = cert["cert"].as_str().expect("a certificate");
        assert!(body.starts_with("ssh-ed25519-cert-v01@openssh.com "), "{case}");
        let principal = cert["principal"].as_str().expect("a principal");
        assert_eq!(principal == ALIAS, case != "wrong_principal", "{case}: {principal}");
        assert!(serials.insert(cert["serial"].as_u64().expect("a serial")), "{case}");
    }
    let revoked: Vec<&str> = certs
        .iter()
        .filter(|c| c["revoked"] == true)
        .map(|c| c["case"].as_str().expect("a case"))
        .collect();
    assert_eq!(revoked, ["revoked"]);
    assert!(!vectors["krl_base64"].as_str().expect("the KRL").is_empty());
}

#[test]
fn the_identity_manifest_schema_requires_the_qualification_fields() {
    let schema = json("identity_manifest.schema.json");
    assert_eq!(schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
    let required: BTreeSet<&str> = items(&schema["required"], "required")
        .iter()
        .map(|r| r.as_str().expect("a field"))
        .collect();
    let fields = ["schema_version", "run", "lab_admin", "candidates", "endpoints", "mtls"];
    assert_eq!(required, fields.into_iter().collect());
    assert_eq!(schema["properties"]["schema_version"]["const"], 1);
    assert_eq!(schema["$defs"]["reference"]["not"]["pattern"], "PRIVATE KEY");
}
