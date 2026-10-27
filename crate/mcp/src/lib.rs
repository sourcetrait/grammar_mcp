pub(crate) mod error;
pub(crate) mod run;
pub(crate) mod server {
    pub(crate) mod cert;
    pub(crate) mod channel {
        pub(crate) mod hub;
        pub(crate) mod state;
    }
    pub(crate) mod emergency;
    pub(crate) mod host;
    pub(crate) mod interface {
        pub(crate) mod contract;
        pub(crate) mod grammar;
    }
    pub(crate) mod rig;
    pub(crate) mod liveness;
    pub(crate) mod namepath;
    pub(crate) mod oneshot;
    pub(crate) mod pin;
    pub(crate) mod purview;
    pub(crate) mod remote {
        pub(crate) mod codec;
        pub(crate) mod link;
        pub(crate) mod verify;
    }
    pub(crate) mod run;
    pub(crate) mod teardown;
    pub(crate) mod watchdog;
    pub(crate) mod tool {
        pub(crate) mod call;
        pub(crate) mod channel_close;
        pub(crate) mod channel_open;
        pub(crate) mod channel_verified;
        pub(crate) mod commit;
        pub(crate) mod config_channel;
        pub(crate) mod common;
        pub(crate) mod handler;
        pub(crate) mod info;
        pub(crate) mod inspect;
        pub(crate) mod interface;
        pub(crate) mod equip;
        pub(crate) mod execute;
        pub(crate) mod kill;
        pub(crate) mod new;
        pub(crate) mod processes;
        pub(crate) mod purview_configure;
        pub(crate) mod purview_extend;
        pub(crate) mod purviews;
        pub(crate) mod purview;
        pub(crate) mod remote_channel_open;
        pub(crate) mod remote_channel_close;
        pub(crate) mod remote_channels;
        pub(crate) mod rerun;
    }
    #[cfg(test)]
    mod tests {
        mod channel;
        mod emergency;
        mod interface;
        mod remote;
        mod rig;
        mod namepath;
        mod pin;
        mod teardown;
        mod watchdog;
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
pub mod guts;

pub(crate) use crate::{
    config::{
        CONFIG, Config, ConfigToml, DEFAULT_NAMESPACE, DenySet, RemoteConfig,
        RemoteRole, SpamThresholds, SupervisorConfig, TEST_NAMESPACE, config, default_id,
        default_work_dir, expand_path, fraction_field,
    },
    home::{base_dirs, cache_base_dir, check_base_dirs, data_base_dir},
    mcp::ServiceExt,
    nu::FromValue,
    error::error_to_call_result,
    server::{
        cert::ensure_cert_profile,
        channel::{
            hub::{FROM_MCP, open_packet, start as start_channel_hub},
            state::{
                ChannelHandle, ChannelPhase, ChannelSendError, ChannelVerifyError,
                CloseSignal as ChannelCloseSignal, EVENT_SPILL_DEST, EVENT_SPILL_KEY,
                MAX_FRAME_BYTES, MCP_RESERVED_PREFIX, SpamVerdict, channel_handle,
                escape_line, event_overflows, event_spill_pointer, inbox_dir, mint_msg_id,
                render_nuon, render_packet,
            },
        },
        emergency::{
            BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency,
            ChannelSpamWarningEmergency, CpuWarningEmergency, CriticalEmergency,
            DiskWarningEmergency, Emergency, EmergencyTx, HungEngineThreadEmergency,
            RamWarningEmergency, VramWarningEmergency, spawn_emergency_responder,
        },
        host::init_engine,
        interface::{
            contract::{InterfaceCommand, parse_command},
            grammar::{
                ArgShape, ArgSpec, COMMANDS, CommandKind, INTERFACE, command_by_words,
                command_spec, interface_engine, render_signatures,
            },
        },
        rig::{
            CommitResult, RigLocks, ValidationResult, check_equip,
            check_source_dir, commit_equip, ensure_substrate, establish_equip,
            render_signatures_within, SignaturesDoc,
            InspectDoc, equip_namepath_of, equip_repo_lock, equip_root, index_node, inspect_impl,
            install_equip, is_reserved_term, is_valid_equip, is_valid_ident,
            is_valid_module_path, lib_dir, load_index, registered_rig_names, require_rig, rig_dir,
            scaffold_leaf, scaffold_leaf_exists, uninstall_equip,
        },
        liveness::acquire as acquire_host_lock,
        namepath::{
            BUILTINS, Namepath, NamepathPattern, NamepathRef, NamepathStr, builtin_equip,
            builtin_name, display_equip, resolve_builtin,
        },
        oneshot::run_oneshot,
        pin::{clear_pins, effective_supervisor, pin, reap_pins},
        purview::{
            CurrentPurview, PURVIEW_ALL, PURVIEW_CORE, PURVIEW_DEFAULT, PURVIEW_REF, PurviewRow,
            PurviewView, is_derived_purview, is_rig_pattern, is_valid_purview_id, load_purviews,
            ensure_default_purview, expand_values, is_nameable_purview, is_valid_purview_ref,
            parse_patterns, pattern_delta, prune_dangling, purview_ref, purview_views,
            purviews_path, resolve_patterns, save_purviews, seed_builtin,
        },
        remote::{
            codec::{
                AcceptorToInitiator, BitcodeCodec, FileFrame, InitiatorToAcceptor, MsgFrame,
                RemoteStream,
            },
            link::{
                RemoteLinkHandle, bound_listeners, close_listener, find_link_send,
                open_remote_blocking, remote_links, safe_dest,
            },
            verify::PublicKeyVerifier,
        },
        run::run_server,
        teardown::{
            OrphanReaper, install_child_subreaper, kill_plugin_subprocesses, make_tracker,
            process_start_time, tree_kill,
        },
        watchdog::{
            HungRegistry, HungWatch, WatchdogDeps, register_hung, spawn_watchdog,
        },
        tool::{
            call::CallParams,
            commit::CommitParams,
            config_channel::ConfigChannelParams,
            equip::EquipParams,
            common::{
                DEFAULT_TIMEOUT_MS, InFlightEntry, InFlightKind, NuSh, RemoteLinkEntry,
                dispatch_pooled, dispatch_stateful, envelope_text, now_millis, serde_to_nu,
                success, success_value, success_void, teardown_all_in_flight,
            },
            inspect::InspectParams,
            interface::{INTERFACE_TOOL, InterfaceParams, interface_description},
            kill::KillParams,
            new::NewParams,
            execute::{ExecuteParams, prepare_execute},
            purview_configure::PurviewConfigureParams,
            purview_extend::PurviewExtendParams,
            purview::PurviewParams,
            remote_channel_close::RemoteChannelCloseParams,
            remote_channel_open::RemoteChannelOpenParams,
            rerun::RerunParams,
        },
    },
    run::{
        parse_deniable,
    },
};

pub(crate) use sourcetrait_grammar_engine::{
    self as engine, CacheKind, ChannelSend, DEFINITION_FILE, Diagnostic, EVAL_STACK_SIZE,
    EngineError, EngineHost, EngineParams, ExecuteDef, Executor, LINT_VIOLATION_CAP, Lane,
    LintEngine, NU_VERSION, ParseEngine, PluginInfo, RemoteSend, Severity, Source,
    StatefulEngine, analyze_definition, args_schema_to_signature, build_call_source,
    build_execute_source, cache_dir, check_value, definition_file, detect_import_cycle,
    ensure_plugin_registry, eval_concurrency_cap, eval_stateless, file_start,
    json_value_to_nu_value, lint_block, list_registered_plugins, nu_to_args_schema,
    nu_to_result_schema, result_schema_to_signature, span_text, span_to_line_col,
    typedef_text, wrap_as_module,
};

pub(crate) use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs,
    io::{self, Write},
    hash::{Hash},
    marker::PhantomData,
    os::unix::fs::OpenOptionsExt,
    path::{PathBuf,Path},
    process,
    sync::{
        Arc, LazyLock, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use clap::Parser;

pub(crate) use sourcetrait_cert_lib as lib_cert;

pub(crate) mod dirs {
    pub(crate) use directories::BaseDirs;
}

pub(crate) mod nu {
    pub(crate) use nu_command::tls::CRYPTO_PROVIDER;
    pub(crate) use nu_engine::command_prelude::Call;
    pub(crate) use nu_json::Value as JsonValue;
    pub(crate) use nu_parser::parse;
    pub(crate) use nu_parser::{FlatShape, flatten_block};
    pub(crate) use nu_protocol::{
        BlockId, Category, DeclId, FromValue, Module, PipelineData, Record,
        ShellError, Signals, Signature, Span, SyntaxShape, Type, Value, VarId,
        ast::{Argument, Block, Expr, Expression, ListItem},
        engine::{Command, EngineState, Job, Jobs, Mail, Stack, StateWorkingSet, ThreadJob},
    };
    pub(crate) use nu_protocol::shell_error::generic::GenericError;
    pub(crate) use nuon::{ToNuonConfig, from_nuon, to_nuon};
}

pub(crate) use sourcetrait_common::{
    cereal,
    datum,
};

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize, de::DeserializeOwned};
}

pub(crate) mod schema {
    pub(crate) use schemars::JsonSchema;
}

pub(crate) mod mcp {
    pub(crate) use rmcp::{
        ErrorData, ServerHandler, ServiceExt,
        handler::server::router::tool::ToolRouter,
        handler::server::wrapper::Parameters,
        model::{
            CallToolResult, ContentBlock, Implementation, JsonObject, ServerCapabilities,
            ServerConfig,
        },
        tool, tool_handler, tool_router,
        transport::stdio,
    };
}

pub(crate) mod tls {
    pub(crate) use tokio_rustls::{TlsAcceptor, TlsConnector, client, server};
    pub(crate) use tokio_rustls::rustls::{ClientConfig, ServerConfig};
    pub(crate) use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
}

pub(crate) mod rv {
    pub(crate) use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, SignatureScheme};
    pub(crate) use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    pub(crate) use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
    pub(crate) use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
    pub(crate) use rustls::crypto::ring::default_provider;
    pub(crate) use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
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
        spawn,
        io::{ReadHalf, WriteHalf, split},
        net::{TcpListener, TcpStream},
        signal::unix::{SignalKind, signal},
        task::{JoinHandle, spawn_blocking},
        sync::{
            Mutex as AsyncMutex, RwLock as AsyncRwLock, Semaphore, oneshot,
            mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        },
        time::{Duration as TkDuration, sleep, timeout},
    };
}

pub(crate) mod tku {
    pub(crate) use tokio_util::codec::{Decoder, Encoder, FramedRead, FramedWrite, LengthDelimitedCodec};
    pub(crate) use tokio_util::bytes::{Bytes, BytesMut};
    pub(crate) use tokio_util::sync::CancellationToken;
}

pub(crate) mod json {
    pub(crate) use serde_json::{Value, from_value, to_value, to_vec};
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
