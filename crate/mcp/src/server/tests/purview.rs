use crate::*;

#[test]
fn a_rig_pattern_is_found_in_either_spelling() {
    assert!(is_rig_pattern("gram:", "sourcetrait/gram"));
    assert!(is_rig_pattern("sourcetrait/gram:", "sourcetrait/gram"));
    assert!(!is_rig_pattern("gram:log:", "sourcetrait/gram"));
    assert!(!is_rig_pattern("acme/gram:", "sourcetrait/gram"));
    assert!(!is_rig_pattern("@gram", "sourcetrait/gram"));
}
