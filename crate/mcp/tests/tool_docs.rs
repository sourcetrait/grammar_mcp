//! The tool surface's documentation: User's two schemas, verbatim, and no tool
//! declaring an output schema.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::TestServer;

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

const EXECUTE_DOC: &str = include_str!("../../../include/human/mcp/execute.md");
const MCP_DOC: &str = include_str!("../../../include/human/mcp/mcp.md");

fn description(s: &TestServer, name: &str) -> String {
    let tools = s.tools();
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("`{name}` is not routed"));
    tool["description"].as_str().unwrap_or_default().to_string()
}

#[test]
fn the_nu_tool_is_described_by_the_execute_contract() {
    let s = TESTING.harness();
    assert_eq!(description(s, "nu"), EXECUTE_DOC);
}

#[test]
fn rerun_names_the_tool_it_reruns() {
    let s = TESTING.harness();
    assert!(description(s, "rerun").contains("`nu`"));
}

#[test]
fn the_instructions_open_with_the_envelope_schema() {
    let s = TESTING.harness();
    let instructions = s.instructions();
    assert!(instructions.starts_with(MCP_DOC), "{instructions}");
    assert!(instructions.contains("/nu/<nonce>/definition.nu"), "{instructions}");
}

#[test]
fn no_tool_declares_an_output_schema() {
    let s = TESTING.harness();
    for tool in s.tools() {
        assert!(tool.get("outputSchema").is_none(), "{tool}");
    }
}
