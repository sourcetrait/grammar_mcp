//! The client advertises nothing before its gates.

use sourcetrait_grammar_protocol::Capabilities;

#[test]
fn the_client_advertises_no_capability() {
    assert_eq!(sourcetrait_grammar_client::capabilities(), Capabilities::NONE);
}
