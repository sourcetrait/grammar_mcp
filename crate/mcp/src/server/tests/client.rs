//! The serving client in process: its four tools, and the runtime's operations
//! answered unavailable in the existing envelope.
use crate::server::client::ClientSh;
use crate::*;

/// The kind a runtime-owned operation answers while no runtime is reachable.
const UNAVAILABLE: &str = "daemon::unavailable";

/// A client on the default config, seeded once for the binary.
fn client() -> ClientSh {
    CONFIG.get_or_init(Config::default);
    let (nonce_gen, mcp_nom) = process_identity();
    ClientSh::new(nonce_gen, mcp_nom, &config().deny)
}

/// A runtime for a tool's future.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime")
}

/// The first error row's kind in a tool's envelope, none on a success.
fn error_kind(result: Result<mcp::CallToolResult, mcp::ErrorData>) -> Option<String> {
    let result = result.unwrap_or_else(|e| panic!("an rmcp-boundary error: {e:?}"));
    let Some(mcp::ContentBlock::Text(text)) = result.content.first() else {
        panic!("a result without its envelope text: {result:?}");
    };
    let envelope = nu::from_nuon(&text.text, None).expect("the envelope is NUON");
    let rows = envelope.get_data_by_key("error")?;
    let row = rows.as_list().ok()?.first()?.clone();
    row.get_data_by_key("kind").and_then(|kind| kind.as_str().ok().map(str::to_string))
}

/// An `interface` call's first error kind.
fn interface_kind(client: &ClientSh, command: &str) -> Option<String> {
    let p = InterfaceParams {
        host: LOCAL_HOST.to_string(),
        command: command.to_string(),
    };
    error_kind(runtime().block_on(client.interface(mcp::Parameters(p))))
}

#[test]
fn the_client_routes_the_four_tools() {
    let _client = client();
    let mut names: Vec<String> = ClientSh::tool_router()
        .list_all()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["call", "execute", "interface", "rerun"]);
}

#[test]
fn each_runtime_owned_tool_answers_unavailable() {
    let client = client();
    let rt = runtime();
    let executed = rt.block_on(client.execute(mcp::Parameters(ExecuteParams {
        host: LOCAL_HOST.to_string(),
        definition: "def execute [data: nothing]: nothing -> nothing { null }".to_string(),
        data: "null".to_string(),
        timeout: None,
    })));
    assert_eq!(error_kind(executed).as_deref(), Some(UNAVAILABLE));
    let rerun = rt.block_on(client.rerun(mcp::Parameters(RerunParams {
        host: LOCAL_HOST.to_string(),
        nonce: "aaaaaaaaaaa".to_string(),
        data: "null".to_string(),
        timeout: None,
    })));
    assert_eq!(error_kind(rerun).as_deref(), Some(UNAVAILABLE));
    let called = rt.block_on(client.call(mcp::Parameters(CallParams {
        host: LOCAL_HOST.to_string(),
        namepath: "sourcetrait/geo:shape:area".to_string(),
        data: "{width: 3.0, height: 4.0}".to_string(),
        timeout: None,
    })));
    assert_eq!(error_kind(called).as_deref(), Some(UNAVAILABLE));
}

#[test]
fn the_runtime_owned_commands_answer_unavailable() {
    let client = client();
    for command in [
        "interface info",
        "interface help interface",
        "interface channel sync",
        "interface channel config",
        "interface equip check 'equip/sourcetrait/geo'",
    ] {
        assert_eq!(interface_kind(&client, command).as_deref(), Some(UNAVAILABLE), "{command}");
    }
}

#[test]
fn an_unknown_host_and_a_broken_command_are_refused_as_before() {
    let client = client();
    let executed = runtime().block_on(client.execute(mcp::Parameters(ExecuteParams {
        host: "elsewhere".to_string(),
        definition: "def execute [data: nothing]: nothing -> nothing { null }".to_string(),
        data: "null".to_string(),
        timeout: None,
    })));
    assert_eq!(error_kind(executed).as_deref(), Some("host::unknown"));
    assert_eq!(
        interface_kind(&client, "interface info | interface purview list").as_deref(),
        Some("interface::invalid"),
    );
}
