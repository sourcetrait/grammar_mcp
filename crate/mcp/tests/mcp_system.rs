//! mcp's controller supervising: a request reaches the engine through it, the
//! operator holds the namespace's host lock while it runs, and the ordered
//! shutdown stops them both. Its own binary, since the host lock is the namespace's.
#![cfg(feature = "embedded-cli")]

use sourcetrait_grammar_mcp::guts::SupervisedSystem;

#[test]
fn mcp_system_starts_both_components_and_stops_them() {
    let system = SupervisedSystem::start();
    assert!(system.engine_answers(), "a request reaches the engine through mcp's controller");
    assert!(system.host_lock_held(), "the operator holds the host lock while it runs");
    system.shutdown();
    assert!(!system.host_lock_held(), "the shutdown stops the operator, its lock released");
    assert!(
        !system.engine_answers(),
        "the engine stopped after it, so a request answers its failure",
    );
}
