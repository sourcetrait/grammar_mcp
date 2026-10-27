use crate::server::interface::contract::KIND_INTERFACE;
use crate::server::interface::grammar::CommandSpec;
use crate::*;

fn engine() -> nu::EngineState {
    interface_engine(&DenySet::default())
}

fn parse(text: &str) -> Result<InterfaceCommand, Vec<Diagnostic>> {
    parse_command(&engine(), text)
}

/// The kinds and messages of a refusal, panicking on a parse that held.
fn refused(text: &str) -> Vec<(String, String)> {
    match parse(text) {
        Ok(command) => panic!("`{text}` held as {command:?}"),
        Err(rows) => rows.into_iter().map(|row| (row.kind, row.message)).collect(),
    }
}

/// The first refusal's message, every row asserted `interface::invalid`.
fn refusal(text: &str) -> String {
    let rows = refused(text);
    assert!(
        rows.iter().all(|(kind, _)| kind == KIND_INTERFACE),
        "`{text}`: every row is {KIND_INTERFACE}; got {rows:?}",
    );
    rows.first().map(|(_, message)| message.clone()).unwrap_or_default()
}

/// The smallest command a spec takes: each required positional a sample.
fn sample(spec: &CommandSpec) -> String {
    let mut text = spec.name.to_string();
    for arg in spec.positionals.iter().filter(|arg| !arg.optional) {
        text.push_str(match arg.shape {
            ArgShape::String => " 'x'",
            ArgShape::Strings => " ['x']",
            ArgShape::Int => " 1",
        });
    }
    text
}

fn kind_of(command: &InterfaceCommand) -> CommandKind {
    match command {
        InterfaceCommand::Info => CommandKind::Info,
        InterfaceCommand::Help(_) => CommandKind::Help,
        InterfaceCommand::ProcessList => CommandKind::ProcessList,
        InterfaceCommand::ProcessKill(_) => CommandKind::ProcessKill,
        InterfaceCommand::PurviewList => CommandKind::PurviewList,
        InterfaceCommand::PurviewSet(_) => CommandKind::PurviewSet,
        InterfaceCommand::PurviewExtend(_) => CommandKind::PurviewExtend,
        InterfaceCommand::PurviewConfigure(_) => CommandKind::PurviewConfigure,
        InterfaceCommand::ChannelOpen => CommandKind::ChannelOpen,
        InterfaceCommand::ChannelReady => CommandKind::ChannelReady,
        InterfaceCommand::ChannelClose => CommandKind::ChannelClose,
        InterfaceCommand::ChannelConfig(_) => CommandKind::ChannelConfig,
        InterfaceCommand::RemoteOpen(_) => CommandKind::RemoteOpen,
        InterfaceCommand::RemoteClose(_) => CommandKind::RemoteClose,
        InterfaceCommand::RemoteList => CommandKind::RemoteList,
        InterfaceCommand::EquipNew(_) => CommandKind::EquipNew,
        InterfaceCommand::EquipInstall(_) => CommandKind::EquipInstall,
        InterfaceCommand::EquipCheck(_) => CommandKind::EquipCheck,
        InterfaceCommand::EquipUninstall(_) => CommandKind::EquipUninstall,
        InterfaceCommand::EquipScaffold(_) => CommandKind::EquipScaffold,
        InterfaceCommand::EquipCommit(_) => CommandKind::EquipCommit,
    }
}

#[test]
fn every_command_parses_to_its_kind() {
    for spec in COMMANDS {
        let text = sample(spec);
        let command = parse(&text).unwrap_or_else(|rows| panic!("`{text}` refused: {rows:?}"));
        assert_eq!(kind_of(&command), spec.kind, "`{text}`");
    }
}

#[test]
fn no_command_name_holds_an_underscore() {
    for spec in COMMANDS {
        assert!(!spec.name.contains('_'), "{}", spec.name);
        for arg in spec.positionals.iter().chain(spec.flags) {
            assert!(!arg.name.contains('_'), "{}: {}", spec.name, arg.name);
        }
    }
}

#[test]
fn arguments_reach_the_tools_parameters() {
    match parse("interface purview configure 'iter/one' ['acme/geo:', '@core']") {
        Ok(InterfaceCommand::PurviewConfigure(p)) => {
            assert_eq!(p.purview, "iter/one");
            assert_eq!(p.namepaths, vec!["acme/geo:".to_string(), "@core".to_string()]);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface info") {
        Ok(InterfaceCommand::Info) => {}
        other => panic!("got {other:?}"),
    }
    assert!(!refused("interface info ['default']").is_empty(), "info takes no argument");
    match parse("interface channel config --spam-warn-rate 12 --spam-error-window-secs 30") {
        Ok(InterfaceCommand::ChannelConfig(p)) => {
            assert_eq!(p.spam_warn_rate, Some(12));
            assert_eq!(p.spam_error_window_secs, Some(30));
            assert_eq!(p.spam_warn_window_secs, None);
            assert_eq!(p.spam_error_rate, None);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface equip install r#'/tmp/a b/equip/acme/geo'#") {
        Ok(InterfaceCommand::EquipInstall(p)) => assert_eq!(p.source, "/tmp/a b/equip/acme/geo"),
        other => panic!("got {other:?}"),
    }
    match parse("interface help grammar:eye:git") {
        Ok(InterfaceCommand::Help(p)) => assert_eq!(p.namepath, "grammar:eye:git"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn an_unknown_command_is_refused() {
    for text in ["interface", "interface purview", "interface nosuch thing", "ls", "nu -c 'ls'"] {
        let message = refusal(text);
        assert!(message.contains("no `interface` command"), "`{text}`: {message}");
    }
}

#[test]
fn one_command_alone_never_piped() {
    assert!(refusal("interface info; interface purview list").contains("alone"));
    assert!(refusal("interface info\ninterface purview list").contains("alone"));
    assert!(refusal("interface info | interface purview list").contains("never piped"));
    assert!(refusal("").contains("alone"));
}

#[test]
fn only_literals_reach_a_command() {
    for text in [
        "interface help $nu",
        "interface help $env.HOME",
        "interface help $in",
        "interface help $\"x(1)\"",
        "interface purview set [$nu.home-dir]",
        "interface purview set [...['a']]",
    ] {
        assert!(!refused(text).is_empty(), "`{text}` must be refused");
    }
    let rows = refused("interface help (interface info)");
    assert!(!rows.is_empty());
}

#[test]
fn a_redirection_is_refused() {
    assert!(!refused("interface info o> out.txt").is_empty());
    assert!(!refused("interface info e> err.txt").is_empty());
}

#[test]
fn a_missing_or_mistyped_argument_is_refused_at_its_position() {
    let rows = parse("interface help").expect_err("help takes a namepath");
    assert_eq!(rows[0].kind, KIND_INTERFACE);
    assert!(rows[0].message.contains("MissingPositional"), "{rows:?}");
    let rows = parse("interface purview set 'x'").expect_err("purviews is a list");
    assert_eq!(rows[0].kind, KIND_INTERFACE);
    let position = rows[0].source.as_ref().expect("a source").position;
    assert_eq!(position[0], 1, "{rows:?}");
    assert!(position[1] > 1, "{rows:?}");
    let rows = parse("interface channel config --spam-warn-rate abc").expect_err("a rate is an int");
    assert_eq!(rows[0].kind, KIND_INTERFACE);
    match parse("interface process kill ['a']") {
        Ok(InterfaceCommand::ProcessKill(p)) => assert_eq!(p.nonce, "['a']", "a string takes the token"),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn a_flag_is_checked_for_its_range() {
    let message = refusal("interface channel config --spam-warn-rate -1");
    assert!(message.contains("non-negative"), "{message}");
    assert!(!refused("interface channel config --nosuch 1").is_empty());
    assert!(!refused("interface info --help").is_empty());
}

#[test]
fn a_denied_command_is_unknown_and_undocumented() {
    let deny = DenySet::new(vec![DeniableTool::Command("purview configure")]);
    let engine = interface_engine(&deny);
    let rows = parse_command(&engine, "interface purview configure x ['y']")
        .expect_err("a denied command is unknown");
    assert!(rows[0].message.contains("no `interface` command"), "{rows:?}");
    assert!(parse_command(&engine, "interface purview list").is_ok());
    let rendered = render_signatures(&deny);
    assert!(!rendered.contains("`purview configure ["), "{rendered}");
    assert!(rendered.contains("`purview list ["), "{rendered}");
}

#[test]
fn the_signatures_cover_every_command_by_topic() {
    let rendered = render_signatures(&DenySet::default());
    assert!(
        rendered.starts_with(
            "## Command Signatures\nNote: Each begins with `interface `\n### General\n"
        ),
        "{rendered}",
    );
    for spec in COMMANDS {
        let entry = format!("{}\n`{}`\n", spec.summary, spec.signature_text());
        assert!(rendered.contains(&entry), "{}", spec.name);
    }
    assert!(rendered.contains(
        "Set a purview's namepath patterns; an empty list deletes it.\n\
         `purview configure [purview: string, namepaths: list<string>]: nothing -> record`\n"
    ));
    assert!(rendered.contains("--spam-warn-rate: int"), "{rendered}");
    assert!(!rendered.contains("def ") && !rendered.contains("- `"), "{rendered}");
}

#[test]
fn every_summary_fits_a_line_and_every_result_is_a_kind() {
    for spec in COMMANDS {
        assert!(spec.summary.chars().count() <= 80, "{}: {}", spec.name, spec.summary);
        assert!(
            ["record", "table", "nothing"].contains(&spec.result),
            "{}: {}",
            spec.name,
            spec.result,
        );
    }
}

#[test]
fn deny_names_a_command_by_its_words() {
    assert_eq!(
        DeniableTool::from_name("purview configure"),
        Some(DeniableTool::Command("purview configure")),
    );
    assert_eq!(DeniableTool::from_name(" equip commit "), Some(DeniableTool::Command("equip commit")));
    assert_eq!(DeniableTool::from_name("interface"), Some(DeniableTool::Interface));
    assert_eq!(DeniableTool::from_name("purview_configure"), None);
    assert_eq!(DeniableTool::from_name("equip"), None);
}
