//! The client in process: its four tools, and the runtime's answering unavailable.
use crate::server::client::ClientSh;
use crate::*;

/// The kind a runtime-owned operation answers while no runtime is reachable.
const UNAVAILABLE: &str = "daemon::unavailable";

/// A client on the default config, its Channel's system running on `rt`.
fn client(rt: &tokio::runtime::Runtime) -> ClientSh {
    CONFIG.get_or_init(Config::default);
    let (nonce_gen, mcp_nom) = process_identity();
    let face = rt
        .block_on(ChannelFace::start(nonce_gen.clone(), mcp_nom))
        .expect("the Channel's system starts");
    ClientSh::new(face, nonce_gen, &config().deny)
}

/// A runtime for a client and its tools' futures.
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
fn interface_kind(rt: &tokio::runtime::Runtime, client: &ClientSh, command: &str) -> Option<String> {
    let p = InterfaceParams {
        host: LOCAL_HOST.to_string(),
        command: command.to_string(),
    };
    error_kind(rt.block_on(client.interface(mcp::Parameters(p))))
}

#[test]
fn the_client_routes_the_four_tools() {
    let rt = runtime();
    let _client = client(&rt);
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
    let rt = runtime();
    let client = client(&rt);
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
    let rt = runtime();
    let client = client(&rt);
    for command in [
        "interface info",
        "interface help interface",
        "interface process list",
        "interface equip check 'equip/sourcetrait/geo'",
    ] {
        assert_eq!(interface_kind(&rt, &client, command).as_deref(), Some(UNAVAILABLE), "{command}");
    }
}

#[test]
fn the_channel_and_the_remote_channels_are_the_clients_own() {
    let rt = runtime();
    let client = client(&rt);
    assert_eq!(interface_kind(&rt, &client, "interface channel config"), None);
    assert_eq!(
        interface_kind(&rt, &client, "interface channel sync").as_deref(),
        Some("channel::not_open"),
    );
    let remote = interface_kind(&rt, &client, "interface remote channel get 'nobody'");
    assert!(
        remote.as_deref().is_some_and(|kind| kind.starts_with("remote::")),
        "the client reads its own remotes; got {remote:?}",
    );
}

#[test]
fn an_unknown_host_and_a_broken_command_are_refused_as_before() {
    let rt = runtime();
    let client = client(&rt);
    let executed = rt.block_on(client.execute(mcp::Parameters(ExecuteParams {
        host: "elsewhere".to_string(),
        definition: "def execute [data: nothing]: nothing -> nothing { null }".to_string(),
        data: "null".to_string(),
        timeout: None,
    })));
    assert_eq!(error_kind(executed).as_deref(), Some("host::unknown"));
    assert_eq!(
        interface_kind(&rt, &client, "interface info | interface purview list").as_deref(),
        Some("interface::invalid"),
    );
}
