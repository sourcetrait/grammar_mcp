//! The scenario groups the qualification contract requires, as this build has them.
use crate::*;

/// The probe and its version, as its outputs name it.
pub(crate) const PROBE: &str = concat!("grammard-probe ", env!("CARGO_PKG_VERSION"));

/// The required scenario groups by case name, in the contract's order.
pub(crate) const GROUPS: [&str; 14] = [
    "wire",
    "runtime",
    "client",
    "artifacts",
    "service",
    "peer",
    "mcp",
    "native-linux",
    "security",
    "recovery",
    "channels-artifacts",
    "peer-isolation",
    "native-riscv",
    "operations",
];

/// One scenario group as the inventory lists it.
#[derive(ser::Serialize)]
pub(crate) struct Group {
    /// The group's case name.
    case: &'static str,
    /// Whether this build implements the group's scenarios.
    implemented: bool,
    /// The group's enumerated assertions, none before its gate defines them.
    assertions: Vec<&'static str>,
}

/// The probe's inventory: its schema, its client's capabilities, its groups.
#[derive(ser::Serialize)]
pub(crate) struct Inventory {
    /// The inventory's schema version.
    schema_version: u32,
    /// The probe and its version.
    probe: &'static str,
    /// The capabilities of the client the probe drives.
    capabilities: Capabilities,
    /// Every required group.
    groups: Vec<Group>,
}

/// This build's inventory: every group listed, none implemented.
pub(crate) fn inventory() -> Inventory {
    Inventory {
        schema_version: 1,
        probe: PROBE,
        capabilities: sourcetrait_grammar_client::capabilities(),
        groups: GROUPS
            .iter()
            .map(|&case| Group { case, implemented: false, assertions: Vec::new() })
            .collect(),
    }
}
