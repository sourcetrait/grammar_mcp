//! The daemon fixtures' public files: the trust vectors, each case OpenSSH's
//! own outcome under a placeholder name, and the identity manifest's model,
//! neither holding any private key data.

use std::collections::BTreeSet;
use std::path::PathBuf;

use nutype::{NuField, NuType, NuTypeFile};
use nuval::NuVal;

/// The placeholder every vector names in place of an endpoint.
const ALIAS: &str = "endpoint.invalid";

/// A fixture file's text from `tests/daemon/`.
fn text(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("daemon").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A model from `tests/daemon/`.
fn model(name: &str) -> NuTypeFile {
    NuTypeFile::from_text(&text(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The trust vectors, read back and held to their model.
fn vectors() -> NuVal {
    let value = NuVal::from_nuon(&text("trust_vectors.nuon")).unwrap_or_else(|e| panic!("the vectors: {e}"));
    model("TrustVectors.nutype").ty.check(&value).unwrap_or_else(|e| panic!("the vectors' model: {e}"));
    value
}

/// A record's field.
fn field<'a>(value: &'a NuVal, name: &str) -> &'a NuVal {
    let NuVal::Record(fields) = value else { panic!("not a record: {value:?}") };
    fields.iter().find(|(k, _)| k == name).map(|(_, v)| v).unwrap_or_else(|| panic!("no {name}"))
}

/// A list's items, refused when the value is not one.
fn items<'a>(value: &'a NuVal, what: &str) -> &'a [NuVal] {
    let NuVal::List(items) = value else { panic!("{what} is not a list") };
    items
}

/// A string's text, refused when the value is not one.
fn string<'a>(value: &'a NuVal, what: &str) -> &'a str {
    let NuVal::String(text) = value else { panic!("{what} is not a string") };
    text
}

#[test]
fn the_trust_vectors_hold_public_material_alone() {
    assert!(!text("trust_vectors.nuon").contains("PRIVATE KEY"), "a private key in the public vectors");
    let vectors = vectors();
    assert_eq!(*field(&vectors, "schema_version"), NuVal::Int(1));
    assert_eq!(string(field(&vectors, "alias"), "the alias"), ALIAS);
    let host = string(field(&vectors, "host_key"), "the host key");
    assert!(host.starts_with("ssh-ed25519 "), "{host}");
    for ca in items(field(&vectors, "cas"), "cas") {
        assert!(string(field(ca, "public"), "a CA's key").starts_with("ssh-ed25519 "));
    }
    for client in items(field(&vectors, "clients"), "clients") {
        let public = string(field(client, "public"), "a client's key");
        assert!(public.starts_with("ssh-ed25519 ") || public.starts_with("ssh-rsa "), "{public}");
    }
}

#[test]
fn every_vector_case_observed_its_expected_class() {
    let vectors = vectors();
    let cases = items(field(&vectors, "cases"), "cases");
    let names: BTreeSet<&str> = cases.iter().map(|c| string(field(c, "case"), "a case")).collect();
    assert_eq!(names.len(), cases.len(), "a case name repeats");
    for case in cases {
        assert_eq!(field(case, "observed"), field(case, "expect"), "{:?}", field(case, "case"));
    }
    let classes: BTreeSet<&str> = cases.iter().map(|c| string(field(c, "expect"), "a class")).collect();
    assert_eq!(classes, ["accept", "auth", "forward", "host_trust"].into_iter().collect());
}

#[test]
fn every_known_hosts_line_names_the_placeholder() {
    let vectors = vectors();
    for file in items(field(&vectors, "known_hosts"), "known_hosts") {
        for line in items(field(file, "lines"), "lines") {
            let line = string(line, "a line");
            assert!(line.contains(ALIAS) || line.starts_with("@revoked * "), "{line}");
        }
    }
}

#[test]
fn every_certificate_names_its_case_principal_and_serial() {
    let vectors = vectors();
    let certs = items(field(&vectors, "certs"), "certs");
    let mut serials = BTreeSet::new();
    for cert in certs {
        let case = string(field(cert, "case"), "a case");
        let body = string(field(cert, "cert"), "a certificate");
        assert!(body.starts_with("ssh-ed25519-cert-v01@openssh.com "), "{case}");
        let principal = string(field(cert, "principal"), "a principal");
        assert_eq!(principal == ALIAS, case != "wrong_principal", "{case}: {principal}");
        let NuVal::Int(serial) = field(cert, "serial") else { panic!("{case}: a serial") };
        assert!(serials.insert(*serial), "{case}");
    }
    let revoked: Vec<&str> = certs
        .iter()
        .filter(|c| *field(c, "revoked") == NuVal::Bool(true))
        .map(|c| string(field(c, "case"), "a case"))
        .collect();
    assert_eq!(revoked, ["revoked"]);
    assert!(!string(field(&vectors, "krl_base64"), "the KRL").is_empty());
}

/// A record's or a table's declared fields.
fn fields(ty: &NuType) -> &[NuField] {
    match ty {
        NuType::Record(fields) | NuType::Table(fields) => fields,
        other => panic!("not a record or a table: {other:?}"),
    }
}

/// A declared field's type.
fn declared<'a>(ty: &'a NuType, name: &str) -> &'a NuType {
    fields(ty).iter().find(|f| f.name == name).map(|f| &f.ty).unwrap_or_else(|| panic!("no {name}"))
}

#[test]
fn the_identity_manifest_model_declares_the_qualification_fields() {
    let manifest = model("IdentityManifest.nutype");
    let names: Vec<&str> = fields(&manifest.ty).iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["schema_version", "run", "lab_admin", "candidates", "endpoints", "mtls"]);
    assert_eq!(*declared(&manifest.ty, "schema_version"), NuType::Int);
    let credentials = declared(declared(&manifest.ty, "endpoints"), "credentials");
    assert_eq!(
        *declared(credentials, "file"),
        NuType::OneOf(vec![NuType::Path, NuType::Nothing]),
        "a redacted copy holds its credentials' files null"
    );
}

#[test]
fn a_verification_carries_its_verdict_and_the_state_behind_it() {
    let manifest = model("IdentityManifest.nutype");
    let verified = declared(declared(&manifest.ty, "endpoints"), "verified");
    let names: Vec<&str> = fields(verified).iter().map(|f| f.name.as_str()).collect();
    let state = ["passed", "failed", "unhealthy_controls", "restored", "tunnel_closed", "error", "openssh"];
    assert_eq!(names[0], "verdict");
    assert_eq!(names[1..], state);
    assert_eq!(*declared(verified, "verdict"), NuType::String);
    for name in state {
        let NuType::OneOf(members) = declared(verified, name) else { panic!("{name} is not nullable") };
        assert!(members.contains(&NuType::Nothing), "{name} is not nullable");
    }
    let doc = manifest.doc.join("\n");
    for verdict in ["`pass`", "`fail`", "`incomplete`", "`unrun`"] {
        assert!(doc.contains(verdict), "the model names the verdict {verdict}");
    }
}

#[test]
fn a_redacted_manifest_holds_to_the_model() {
    let redacted = "{schema_version: 1, run: r1, lab_admin: {via: ssh, scope: fixture}, candidates: null, \
        endpoints: [{profile: p, arch: amd64, route: {host: h, bind: b, port: 22, forward_port: 112}, \
        host_key_alias: \"endpoint.invalid\", host_key: \"SHA256:x\", \
        accounts: [{user: u, uid: 1000, keys: []}], \
        worker: {id: fixture, namespace: default, instance_key: k, socket: s}, \
        trust: {host_ca: a, unknown_ca: b, krl_sha256: c, certs: []}, \
        credentials: [{name: n, file: null, fingerprint: f}], \
        verified: {verdict: unrun, passed: null, failed: null, unhealthy_controls: null, \
        restored: null, tunnel_closed: null, error: null, openssh: null}}], mtls: null}";
    let value = NuVal::from_nuon(redacted).expect("NUON");
    model("IdentityManifest.nutype").ty.check(&value).expect("a redacted manifest holds");
}
