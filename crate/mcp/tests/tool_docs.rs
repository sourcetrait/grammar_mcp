//! The tool surface's documentation: User's three texts, verbatim, and no tool
//! declaring an output schema.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::TestServer;

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

const EXECUTE_DOC: &str = include_str!("../../../include/human/mcp/execute.md");
const MCP_DOC: &str = include_str!("../../../include/human/mcp/mcp.md");
const INTERFACE_DOC: &str = include_str!("../../../include/human/mcp/interface.md");

fn description(s: &TestServer, name: &str) -> String {
    let tools = s.tools();
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("`{name}` is not routed"));
    tool["description"].as_str().unwrap_or_default().to_string()
}

#[test]
fn the_execute_tool_is_described_by_its_contract() {
    let s = TESTING.harness();
    assert_eq!(description(s, "execute"), EXECUTE_DOC);
}

#[test]
fn rerun_names_the_tool_it_reruns() {
    let s = TESTING.harness();
    assert!(description(s, "rerun").contains("`execute`"));
}

#[test]
fn the_instructions_open_with_the_envelope_schema() {
    let s = TESTING.harness();
    let instructions = s.instructions();
    assert!(instructions.starts_with(MCP_DOC), "{instructions}");
    assert!(instructions.contains("/execute/<nonce>/definition.nu"), "{instructions}");
}

#[test]
fn no_tool_declares_an_output_schema() {
    let s = TESTING.harness();
    for tool in s.tools() {
        assert!(tool.get("outputSchema").is_none(), "{tool}");
    }
}

#[test]
fn four_tools_are_routed() {
    let s = TESTING.harness();
    let mut names: Vec<String> = s
        .tools()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["call", "execute", "interface", "rerun"]);
}

#[test]
fn the_interface_is_described_by_its_pretext_then_its_command_signatures() {
    let s = TESTING.harness();
    let description = description(s, "interface");
    let (pretext, signatures) = description
        .split_once("\n\n## Command Signatures\n")
        .unwrap_or_else(|| panic!("the signatures follow the pretext: {description}"));
    assert_eq!(pretext, INTERFACE_DOC.trim_end(), "{description}");
    assert!(
        signatures.starts_with("Note: Each begins with `interface `\n"),
        "{description}",
    );
    for topic in ["General", "Process", "Purview", "Channel", "Remote", "Equip"] {
        assert!(signatures.contains(&format!("### {topic}\n")), "{topic}: {description}");
    }
    assert!(
        signatures.contains(
            "Displays equipment documentation\n`help [namepath: string]: nothing -> record`\n"
        ),
        "{description}",
    );
    assert!(
        signatures.contains(
            "Verifies Channel activation and begins normal use.\n\
             `channel ready []: nothing -> nothing`\n"
        ),
        "{description}",
    );
    let lines = signatures.lines().filter(|line| line.starts_with('`')).count();
    assert_eq!(lines, 21, "{description}");
}

#[test]
fn every_description_fits_claude_codes_cap() {
    let s = TESTING.harness();
    for tool in s.tools() {
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(
            description.chars().count() <= 4096,
            "`{}` is {} characters",
            tool["name"],
            description.chars().count(),
        );
    }
}
