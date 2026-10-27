//! The interface commands: their table, their decls, and their isolated engine.
use crate::*;

/// The tool's command head, the first word of every command.
pub(crate) const INTERFACE: &str = "interface";

/// What one argument holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArgShape {
    String,
    Strings,
    Int,
}

impl ArgShape {
    fn syntax(self) -> nu::SyntaxShape {
        match self {
            Self::String => nu::SyntaxShape::String,
            Self::Strings => nu::SyntaxShape::List(Box::new(nu::SyntaxShape::String)),
            Self::Int => nu::SyntaxShape::Int,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Strings => "list<string>",
            Self::Int => "int",
        }
    }
}

/// One argument: a positional in its order, or a flag by its name.
#[derive(Debug)]
pub(crate) struct ArgSpec {
    pub name: &'static str,
    pub shape: ArgShape,
    pub optional: bool,
    pub summary: &'static str,
}

/// Which command a spec names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandKind {
    Info,
    Help,
    ProcessList,
    ProcessKill,
    PurviewList,
    PurviewSet,
    PurviewExtend,
    PurviewConfigure,
    ChannelOpen,
    ChannelReady,
    ChannelClosed,
    ChannelConfig,
    RemoteOpen,
    RemoteClose,
    RemoteList,
    EquipNew,
    EquipInstall,
    EquipCheck,
    EquipUninstall,
    EquipScaffold,
    EquipCommit,
}

/// One interface command: its name, arguments, result, and summary.
#[derive(Debug)]
pub(crate) struct CommandSpec {
    pub kind: CommandKind,
    pub name: &'static str,
    pub topic: &'static str,
    pub positionals: &'static [ArgSpec],
    pub flags: &'static [ArgSpec],
    pub result: &'static str,
    pub summary: &'static str,
}

impl CommandSpec {
    /// The command's words after `interface`, its name to `--deny`.
    pub(crate) fn words(&self) -> &'static str {
        let name: &'static str = self.name;
        &name[INTERFACE.len() + 1..]
    }

    /// The signature the parse checks the command's arguments against.
    pub(crate) fn signature(&self) -> nu::Signature {
        let mut signature = nu::Signature::build(self.name)
            .input_output_type(nu::Type::Nothing, nu::Type::Any)
            .category(nu::Category::Custom(INTERFACE.to_string()));
        for arg in self.positionals {
            signature = if arg.optional {
                signature.optional(arg.name, arg.shape.syntax(), arg.summary)
            } else {
                signature.required(arg.name, arg.shape.syntax(), arg.summary)
            };
        }
        for flag in self.flags {
            signature = signature.named(flag.name, flag.shape.syntax(), flag.summary, None);
        }
        signature
    }

    /// The command as one `def` line: its parameters and its result's type.
    pub(crate) fn signature_text(&self) -> String {
        let positionals = self.positionals.iter().map(|arg| {
            let optional = if arg.optional { "?" } else { "" };
            format!("{}{optional}: {}", arg.name, arg.shape.text())
        });
        let flags = self
            .flags
            .iter()
            .map(|flag| format!("--{}: {}", flag.name, flag.shape.text()));
        let parameters: Vec<String> = positionals.chain(flags).collect();
        format!(
            "def '{}' [{}]: nothing -> {}",
            self.name,
            parameters.join(", "),
            self.result,
        )
    }
}

/// A diagnostic row's type, as the refusing commands' warnings carry it.
macro_rules! diagnostics_type {
    () => {
        "table<kind: string, source: oneof<nothing, record<path: oneof<nothing, string>, \
         position: list<int>>>, message: string>"
    };
}

const NO_ARGS: &[ArgSpec] = &[];

const INFO_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "purviews",
    shape: ArgShape::Strings,
    optional: true,
    summary: "Purview ids to render as if in view; empty means the current view.",
}];

const HELP_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "namepath",
    shape: ArgShape::String,
    optional: false,
    summary: "A namepath at any arity, or a pattern ending in `/` or `:`, or `*` / `.`.",
}];

const KILL_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "nonce",
    shape: ArgShape::String,
    optional: false,
    summary: "The nonce of the in-flight call to cancel.",
}];

const SET_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "purviews",
    shape: ArgShape::Strings,
    optional: false,
    summary: "The purview ids to put in view, replacing the current set.",
}];

const EXTEND_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "purviews",
    shape: ArgShape::Strings,
    optional: false,
    summary: "Purview ids to add to the current view.",
}];

const CONFIGURE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "purview",
        shape: ArgShape::String,
        optional: false,
        summary: "The purview to set, created if absent; a path-like snake label.",
    },
    ArgSpec {
        name: "namepaths",
        shape: ArgShape::Strings,
        optional: false,
        summary: "Its namepath patterns; an empty list deletes the purview.",
    },
];

const CONFIG_FLAGS: &[ArgSpec] = &[
    ArgSpec {
        name: "spam-warn-window-secs",
        shape: ArgShape::Int,
        optional: true,
        summary: "Window for the soft threshold, in whole seconds.",
    },
    ArgSpec {
        name: "spam-warn-rate",
        shape: ArgShape::Int,
        optional: true,
        summary: "Sends within that window before one warning is issued for an origin.",
    },
    ArgSpec {
        name: "spam-error-window-secs",
        shape: ArgShape::Int,
        optional: true,
        summary: "Window for the hard threshold, in whole seconds.",
    },
    ArgSpec {
        name: "spam-error-rate",
        shape: ArgShape::Int,
        optional: true,
        summary: "Sends within that window before the origin is refused and stopped.",
    },
];

const OPEN_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "alias",
    shape: ArgShape::String,
    optional: false,
    summary: "The `[[remote]]` alias to open, from `.grammar/mcp/remotes.toml`.",
}];

const CLOSE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "alias",
    shape: ArgShape::String,
    optional: false,
    summary: "The alias of the open link to close.",
}];

const SOURCE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "source",
    shape: ArgShape::String,
    optional: false,
    summary: "The equip's source: an `equip/<author>/<name>` directory.",
}];

const SCAFFOLD_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "namepaths",
    shape: ArgShape::Strings,
    optional: false,
    summary: "Namepaths to scaffold into installed equips' rigs: a module or a function.",
}];

const COMMIT_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "namepath",
    shape: ArgShape::String,
    optional: false,
    summary: "The equip to commit: its `<author>/<name>`, or a builtin's name.",
}];

/// Every interface command, in the order the description lists them.
pub(crate) const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        kind: CommandKind::Info,
        name: "interface info",
        topic: "General",
        positionals: INFO_ARGS,
        flags: NO_ARGS,
        result: "record<name: string, version: string, nu_version: string, id: string, \
                 namespace: string, mcp_nom: string, work_dir: string, \
                 plugins: list<list<oneof<string, nothing>>>, signatures: string, \
                 purview: list<list<oneof<string, list<string>>>>>",
        summary: "Versions, plugins, and every rig's callable signatures.",
    },
    CommandSpec {
        kind: CommandKind::Help,
        name: "interface help",
        topic: "General",
        positionals: HELP_ARGS,
        flags: NO_ARGS,
        result: "record<doc: oneof<record<srcdir: string, summary: string, details: string>, \
                 record<src: string, summary: string, details: string>, \
                 record<src: string, signature: string, details: string>, \
                 record<signatures: string>>>",
        summary: "Detailed documentation for rigs, modules, and calls.",
    },
    CommandSpec {
        kind: CommandKind::ProcessList,
        name: "interface process list",
        topic: "Process",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record<processes: table<nonce: string, tool: string, started_at: int, \
                 args: record, source_nonce: oneof<nothing, string>, \
                 path: oneof<nothing, string>>>",
        summary: "List in-flight MCP tool usage.",
    },
    CommandSpec {
        kind: CommandKind::ProcessKill,
        name: "interface process kill",
        topic: "Process",
        positionals: KILL_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Cancel an in-flight usage by its nonce.",
    },
    CommandSpec {
        kind: CommandKind::PurviewList,
        name: "interface purview list",
        topic: "Purview",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record<purviews: list<list<oneof<string, list<string>>>>, current: list<string>>",
        summary: "List every configured purview, and what is currently in view.",
    },
    CommandSpec {
        kind: CommandKind::PurviewSet,
        name: "interface purview set",
        topic: "Purview",
        positionals: SET_ARGS,
        flags: NO_ARGS,
        result: "record<revealed: oneof<nothing, string>>",
        summary: "Set the current view to these purviews.",
    },
    CommandSpec {
        kind: CommandKind::PurviewExtend,
        name: "interface purview extend",
        topic: "Purview",
        positionals: EXTEND_ARGS,
        flags: NO_ARGS,
        result: "record<revealed: oneof<nothing, string>, current: list<string>>",
        summary: "Bring more purviews into the current view.",
    },
    CommandSpec {
        kind: CommandKind::PurviewConfigure,
        name: "interface purview configure",
        topic: "Purview",
        positionals: CONFIGURE_ARGS,
        flags: NO_ARGS,
        result: "record<signatures: oneof<nothing, string>>",
        summary: "Set a purview's namepath patterns; an empty list deletes it.",
    },
    CommandSpec {
        kind: CommandKind::ChannelOpen,
        name: "interface channel open",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record<status: string, wss: string, inbox: string>",
        summary: "Open the host's packet channel and return the endpoint to watch.",
    },
    CommandSpec {
        kind: CommandKind::ChannelReady,
        name: "interface channel ready",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Confirm the channel/Open packet was seen; ends the verify window.",
    },
    CommandSpec {
        kind: CommandKind::ChannelClosed,
        name: "interface channel closed",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Close the host's packet channel.",
    },
    CommandSpec {
        kind: CommandKind::ChannelConfig,
        name: "interface channel config",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: CONFIG_FLAGS,
        result: "record<spam_warn_window_secs: int, spam_warn_rate: int, \
                 spam_error_window_secs: int, spam_error_rate: int>",
        summary: "Read or adjust the channel's send-rate thresholds at runtime.",
    },
    CommandSpec {
        kind: CommandKind::RemoteOpen,
        name: "interface remote open",
        topic: "Remote",
        positionals: OPEN_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Open a configured remote link by alias; blocks on bind (listener) or connect \
                  (connector, 20s) and returns synchronously - null on success, an error \
                  envelope on failure. A listener then emits mcp/remote/Connected when a peer \
                  pairs; Disconnected fires on any established-link teardown.",
    },
    CommandSpec {
        kind: CommandKind::RemoteClose,
        name: "interface remote close",
        topic: "Remote",
        positionals: CLOSE_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Close an open mTLS link to a remote grammar host.",
    },
    CommandSpec {
        kind: CommandKind::RemoteList,
        name: "interface remote list",
        topic: "Remote",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record<channels: table<alias: string, remote_mcp_nom: string, address: string>, \
                 listening: table<remote: string, address: string>>",
        summary: "List remote grammar-host links: established `channels` plus \
                  bound-but-unpaired `listening` listeners.",
    },
    CommandSpec {
        kind: CommandKind::EquipNew,
        name: "interface equip new",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: "record<summary: record<created: list<string>>>",
        summary: "Register a new equip at its source and seed its rig.",
    },
    CommandSpec {
        kind: CommandKind::EquipInstall,
        name: "interface equip install",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: concat!(
            "record<summary: record<added: list<string>, modified: list<string>, \
             removed: list<string>, warnings: ",
            diagnostics_type!(),
            ">>"
        ),
        summary: "Validate an equip's whole source and install it.",
    },
    CommandSpec {
        kind: CommandKind::EquipCheck,
        name: "interface equip check",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: concat!(
            "record<summary: record<ok: bool, errors: ",
            diagnostics_type!(),
            ", warnings: ",
            diagnostics_type!(),
            ">>"
        ),
        summary: "Validate an installed equip's source, writing nothing.",
    },
    CommandSpec {
        kind: CommandKind::EquipUninstall,
        name: "interface equip uninstall",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: "record<>",
        summary: "Remove an installed equip from the namespace; its source stays.",
    },
    CommandSpec {
        kind: CommandKind::EquipScaffold,
        name: "interface equip scaffold",
        topic: "Equip",
        positionals: SCAFFOLD_ARGS,
        flags: NO_ARGS,
        result: "record<created: list<string>>",
        summary: "Scaffold modules / functions (by namepath) into an equip's rig.",
    },
    CommandSpec {
        kind: CommandKind::EquipCommit,
        name: "interface equip commit",
        topic: "Equip",
        positionals: COMMIT_ARGS,
        flags: NO_ARGS,
        result: concat!(
            "record<added: list<string>, modified: list<string>, removed: list<string>, \
             warnings: ",
            diagnostics_type!(),
            ">"
        ),
        summary: "Commit an equip's lib and rig from its source to the MCP's repository for \
                  live use.",
    },
];

/// The spec a command's full name names.
pub(crate) fn command_spec(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|spec| spec.name == name)
}

/// The spec a deny entry names by the command's words after `interface`.
pub(crate) fn command_by_words(words: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|spec| spec.words() == words)
}

/// A signature-only decl the parse resolves a command to; it never runs.
#[derive(Clone)]
pub(crate) struct InterfaceDecl {
    spec: &'static CommandSpec,
}

impl nu::Command for InterfaceDecl {
    fn name(&self) -> &str {
        self.spec.name
    }

    fn signature(&self) -> nu::Signature {
        self.spec.signature()
    }

    fn description(&self) -> &str {
        self.spec.summary
    }

    fn run(
        &self,
        _engine_state: &nu::EngineState,
        _stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        Err(nu::GenericError::new(
            format!("`{}` is never evaluated", self.spec.name),
            "an interface command is parsed and dispatched by the host",
            call.head,
        )
        .into())
    }
}

/// An engine holding the commands the deny set leaves, and nothing else.
pub(crate) fn interface_engine(deny: &DenySet) -> nu::EngineState {
    let mut engine_state = nu::EngineState::new();
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    for spec in COMMANDS.iter().filter(|spec| !deny.denies_command(spec.words())) {
        working_set.add_decl(Box::new(InterfaceDecl { spec }));
    }
    let delta = working_set.render();
    let _ = engine_state.merge_delta(delta);
    engine_state
}

/// The signatures the deny set leaves, a `###` heading a topic.
pub(crate) fn render_signatures(deny: &DenySet) -> String {
    let mut out = String::from("## Command Signatures\n");
    let mut topic = "";
    for spec in COMMANDS.iter().filter(|spec| !deny.denies_command(spec.words())) {
        if spec.topic != topic {
            topic = spec.topic;
            out.push_str(&format!("### {topic}\n"));
        }
        out.push_str(&format!("- `{}` {}\n", spec.signature_text(), spec.summary));
    }
    out
}
