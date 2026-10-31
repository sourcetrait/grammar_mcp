//! The capability set: nothing on before the gates, and it crosses the codec.

use sourcetrait_grammar_protocol::{Capabilities, DECODE_BUDGET_DEFAULT, decode_payload};

#[test]
fn no_capability_is_on_before_its_gate() {
    let none = Capabilities::NONE;
    assert!(!none.wire && !none.runtime && !none.client && !none.artifacts && !none.service);
}

#[test]
fn capabilities_round_trip_through_bitcode() {
    let some = Capabilities { client: true, ..Capabilities::NONE };
    let bytes = bitcode::encode(&some);
    let back: Capabilities = decode_payload(&bytes, DECODE_BUDGET_DEFAULT).expect("decoded");
    assert_eq!(back, some);
    assert_ne!(back, Capabilities::NONE);
}
