#![cfg_attr(not(feature = "embedded-cli"), allow(dead_code, unused_imports))]

pub(crate) mod error;
pub(crate) mod run;
pub(crate) mod server {
    pub(crate) mod cert;
    pub(crate) mod channel {
        pub(crate) mod bridge;
        pub(crate) mod face;
        pub(crate) mod hub;
    }
    pub(crate) mod client;
    pub(crate) mod emergency;
    #[cfg(feature = "embedded-cli")]
    pub(crate) mod host;
    pub(crate) mod identity;
    pub(crate) mod interface {
        pub(crate) mod contract;
        pub(crate) mod grammar;
    }
    #[cfg(feature = "embedded-cli")]
    pub(crate) mod oneshot;
    #[cfg(feature = "embedded-cli")]
    pub(crate) mod operator;
    #[cfg(feature = "embedded-cli")]
    pub(crate) mod purview;
    #[cfg(feature = "embedded-cli")]
    pub(crate) mod run;
    #[cfg(feature = "embedded-cli")]
    pub(crate) mod system;
    pub(crate) mod tool {
        pub(crate) mod call;
        pub(crate) mod channel_close;
        pub(crate) mod channel_open;
        pub(crate) mod channel_sync;
        pub(crate) mod channel_verified;
        pub(crate) mod commit;
        pub(crate) mod config_channel;
        pub(crate) mod common;
        #[cfg(feature = "embedded-cli")]
        pub(crate) mod handler;
        #[cfg(feature = "embedded-cli")]
        pub(crate) mod info;
        pub(crate) mod inspect;
        pub(crate) mod interface;
        pub(crate) mod equip;
        pub(crate) mod execute;
        pub(crate) mod kill;
        pub(crate) mod new;
        #[cfg(feature = "embedded-cli")]
        pub(crate) mod processes;
        pub(crate) mod purview_configure;
        pub(crate) mod purview_extend;
        #[cfg(feature = "embedded-cli")]
        pub(crate) mod purviews;
        pub(crate) mod purview;
        pub(crate) mod remote_channel_close;
        pub(crate) mod remote_channel_connect;
        pub(crate) mod remote_channel_listen;
        pub(crate) mod remote_channel_set;
        pub(crate) mod remote_channels;
        pub(crate) mod rerun;
    }
    #[cfg(test)]
    mod tests {
        mod client;
        mod emergency;
        mod face;
        mod interface;
        #[cfg(feature = "embedded-cli")]
        mod purview;
    }
}
#[cfg(test)]
mod tests {
    mod config;
}
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod consts;
pub(crate) mod home;
pub(crate) mod scratch;
#[cfg(feature = "embedded-cli")]
pub mod guts;

pub(crate) use crate::{
    config::{
        AGENT_CONFIG_FILE, CONFIG, Config, DEFAULT_NAMESPACE, DenySet, TEST_NAMESPACE, config,
        default_id, default_work_dir, expand_defaults, expand_path,
    },
    home::{
        TMP_DEFAULT, base_dirs, cache_base_dir, check_base_dirs, data_base_dir, env_dir,
        home_var, home_vars, uenv_partner, uenv_standard,
    },
    scratch::{
        make_channel_dir, make_scratch_dirs, remove_channel_dir, remove_scratch_dirs, tmp_dir,
        transient_dir,
    },
    mcp::ServiceExt,
    error::error_to_call_result,
    server::{
        cert::ensure_cert_profile,
        channel::{
            bridge::{host_channel, set_host_channel},
            face::{
                ChannelFace, CloseSignal as ChannelCloseSignal, Hub, HubSlot, channel_failed,
                peer_failed, spawn_signal_sweep,
            },
            hub::{FROM_MCP, open_packet, start as start_channel_hub},
        },
        client::serve_client,
        emergency::{EmergencyTx, spawn_emergency_responder},
        identity::{mint_nonce, process_identity},
        interface::{
            contract::{InterfaceCommand, parse_command},
            grammar::{interface_engine, render_command_list, render_signatures},
        },
        tool::{
            call::{CALL_DESCRIPTION, CallParams},
            channel_open::ChannelOpenParams,
            commit::CommitParams,
            config_channel::ConfigChannelParams,
            equip::EquipParams,
            common::{
                envelope_text, now_millis, resolve_host, serde_to_nu, success, success_value,
                success_void,
            },
            inspect::InspectParams,
            interface::{
                INTERFACE_DESCRIPTION, INTERFACE_TOOL, InterfaceParams, interface_description,
            },
            kill::KillParams,
            new::NewParams,
            execute::ExecuteParams,
            purview_configure::PurviewConfigureParams,
            purview_extend::PurviewExtendParams,
            purview::PurviewParams,
            remote_channel_close::RemoteChannelCloseParams,
            remote_channel_connect::RemoteConnectParams,
            remote_channel_listen::RemoteListenParams,
            remote_channel_set::RemoteSetParams,
            remote_channels::RemoteAliasParams,
            rerun::{RERUN_DESCRIPTION, RerunParams},
        },
    },
    run::{
        parse_deniable,
    },
};

#[cfg(feature = "embedded-cli")]
pub(crate) use crate::server::{
    host::{HostRoute, ensure_substrate, init_engine},
    oneshot::run_oneshot,
    operator::{OperatorWatch, spawn_observer},
    purview::{
        CurrentPurview, PURVIEW_ALL, PURVIEW_CORE, PURVIEW_DEFAULT, PURVIEW_REF,
        is_derived_purview, is_rig_pattern, is_valid_purview_id, load_purviews,
        ensure_default_purview, expand_values, is_nameable_purview, is_valid_purview_ref,
        pattern_delta, prune_dangling, purview_ref, purview_views,
        purviews_path, resolve_patterns, save_purviews, seed_builtin, seed_builtin_purviews,
    },
    run::run_server,
    system::{FromMcpSys, McpHandle, McpSystem, Notice, ToMcpSys, notice_status, start_system},
    tool::common::{NuSh, answer_value, unexpected},
};

#[cfg(feature = "embedded-cli")]
pub(crate) use sourcetrait_grammar_engine::{
    self as engine, EVAL_STACK_SIZE, EngineHandle, EngineParams, EngineSystem, FromEngine,
    GRIMM, HostAnswer, HostAsk, HostStop, LintEngine, NU_VERSION, NamepathPattern,
    NamepathStr, ParseEngine, PinAsk, PluginInfo, ToEngine, analyze_definition, build_call_source,
    build_execute_source, definition_file, ensure_equip_store, ensure_plugin_registry,
    equip_repo_lock, equip_root, file_start, grimm_signatures, index_from_nuon,
    install_builtins, lib_dir, lint_block, list_registered_plugins,
    registered_rig_names, rig_dir, rig_meta_path, typedef_text,
};

pub(crate) use sourcetrait_grammar_channel::{
    ChannelHandle, ChannelSendError, ChannelStatus, ChannelSyncError, ChannelSysParams,
    ChannelSysPaths, ChannelSystem, ChannelVerifyError, CloseHub, CloseRequest, EmitError,
    EmitRequest, ExpireRequest, FromChannelSys, HostRequest, HubNotice, Line, OpenRequest,
    ReadyRequest, SendRequest, SendVerdict, SpamThresholds, StatusRequest, StoppedNotice,
    SyncRequest, ThresholdsRequest, escape_line, mint_msg_id, render_nuon, render_packet,
};

pub(crate) use sourcetrait_grammar_peer::{
    ExpandDefault, FromPeerSys, IdentityKeys, InboxRequest, Payload, PeerHandle, PeerSysParams,
    PeerSysPaths, PeerSystem, REMOTES_FILE, Relay, RemoteCloseRequest, RemoteConnectRequest,
    RemoteGetRequest, RemoteListRequest, RemoteListenRequest, RemoteSendRequest,
    RemoteSetRequest, RemoteUnsetRequest, safe_dest,
};

pub(crate) use sourcetrait_grammar_protocol::{
    ArtifactError, ChannelEpoch, ChannelPhase, ChannelSendData, LOCAL_HOST, MAX_FRAME_BYTES,
    PurviewRow, PurviewView, RemoteResult, RemoteSendData, RuntimeError, SpamThresholdsData,
    TransportError,
};

pub(crate) use sourcetrait_grammar_protocol::{
    ArgShape, ArgSpec, COMMANDS, CallRequest, CheckSummary, CommandKind, CommandSpec,
    CommitResult, ControlAnswer, ControlRequest, Diagnostic, EngineCommand, EngineError,
    EngineHealth, EngineResult, ExecuteRequest, FromEngineSys, Host, INTERFACE, InspectDoc,
    LINT_VIOLATION_CAP, RerunRequest, SignaturesDoc, Source, ToEngineSys, ValueAnswer,
    builtin_equip, builtin_name, command_by_words, command_spec, span_to_line_col,
};

pub(crate) use sourcetrait_grammar_protocol::{Emergency, EmergencyKind, Observation};

#[cfg(feature = "embedded-cli")]
pub(crate) use sourcetrait_grammar_operator::{
    FromOperatorSys, OperatorHandle, OperatorSysParams, OperatorSysPaths, OperatorSystem,
    SAMPLE_INTERVAL, SupervisorConfig, check_fraction, clear_pins, effective_supervisor,
    install_child_subreaper, pin,
};

pub(crate) use nuval::{NuVal, NuonStyle, TryFromNu, TryIntoNu, wide};

pub(crate) use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::{PathBuf,Path},
    process,
    sync::{
        Arc, LazyLock, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use clap::Parser;

pub(crate) use srcert_lib as lib_cert;

pub(crate) mod dirs {
    pub(crate) use directories::BaseDirs;
}

pub(crate) mod nu {
    pub(crate) use nu_engine::command_prelude::Call;
    pub(crate) use nu_parser::parse;
    pub(crate) use nu_protocol::{
        Category, PipelineData, Record, ShellError, Signature, Span, SyntaxShape,
        Type, Value,
        ast::{Argument, Block, Expr, Expression, ListItem},
        engine::{Command, EngineState, Stack, StateWorkingSet},
    };
    pub(crate) use nu_protocol::shell_error::generic::GenericError;
    pub(crate) use nuon::{ToNuonConfig, from_nuon, to_nuon};
}

pub(crate) use sourcetrait_common::{agnostic, cereal, datum, subsys};

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub(crate) mod schema {
    pub(crate) use schemars::JsonSchema;
}

pub(crate) mod mcp {
    pub(crate) use rmcp::{
        ErrorData, ServerHandler, ServiceExt,
        handler::server::router::tool::ToolRouter,
        handler::server::wrapper::Parameters,
        model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
        tool, tool_handler, tool_router,
        transport::stdio,
    };
}

pub(crate) mod tls {
    pub(crate) use tokio_rustls::TlsAcceptor;
    pub(crate) use tokio_rustls::rustls::ServerConfig;
    pub(crate) use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
}

pub(crate) mod ws {
    pub(crate) use tokio_tungstenite::accept_async;
    pub(crate) use tokio_tungstenite::tungstenite::Message;
    pub(crate) use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    pub(crate) use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
}

pub(crate) use futures_util::{SinkExt, StreamExt};
pub(crate) use tokio_rustls::rustls::pki_types::pem::PemObject;

pub(crate) mod tk {
    pub(crate) use tokio::{
        select, spawn,
        net::{TcpListener, TcpStream},
        task::{JoinHandle, spawn_blocking},
        sync::{
            Mutex as AsyncMutex, oneshot,
            mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        },
        time::{Duration as TkDuration, sleep, timeout},
    };
    #[cfg(unix)]
    pub(crate) use tokio::signal::unix::{SignalKind, signal};
}

pub(crate) mod json {
    pub(crate) use serde_json::{Value, to_value};
}

pub use crate::{
    cli::{
        Cli, CliEnv, CliTool, CliCmd,
    },
    config::{
        DeniableTool,
    },
    error::{
        GrammarMcpResult, GrammarMcpError,
    },
    run::{
        run_main, run_with
    },
};

pub use sourcetrait_grammar_protocol::ChannelMode;
