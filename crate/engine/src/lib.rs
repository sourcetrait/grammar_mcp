pub(crate) mod blocked;
pub(crate) mod cache;
pub(crate) mod config_dirs;
pub(crate) mod cycle;
pub(crate) mod embed;
pub(crate) mod engine;
pub(crate) mod error;
pub(crate) mod execute;
pub(crate) mod executor;
pub(crate) mod host;
pub(crate) mod lint;
pub(crate) mod mode;
pub(crate) mod nuapi {
    pub(crate) mod grimm {
        pub(crate) mod category;
        pub(crate) mod channel {
            pub(crate) mod send;
        }
        pub(crate) mod common;
        pub(crate) mod config;
        pub(crate) mod dbg;
        pub(crate) mod def;
        pub(crate) mod remote {
            pub(crate) mod channel {
                pub(crate) mod send;
            }
        }
    }
    pub(crate) mod print;
}
pub(crate) mod nuin;
pub(crate) mod params;
pub(crate) mod parse_engine;
pub(crate) mod plugins;
pub(crate) mod schema;
pub(crate) mod template;
#[cfg(feature = "test-hooks")]
pub(crate) mod test_hooks;
#[cfg(test)]
mod tests {
    mod schema;
}

pub(crate) use crate::{
    blocked::shadow_host_fatal_decls,
    cache::call_log_dir,
    config_dirs::{config_dirs, init_config_dirs},
    embed::{eval_in_process, merge_env_no_chdir},
    engine::startup,
    execute::EXECUTE,
    host::{host, set_host},
    mode::Mode,
    nuapi::{
        grimm::{
            category::*,
            channel::send::GrimmChannelSend,
            common::{
                NuapiCall, STDERR_FILE, STDOUT_FILE, data_shape, event_id_record, event_id_type,
                register_nuapi, require_record_or_table,
            },
            config::{GrimmGetConfig, GrimmGetConfigAll, GrimmPinConfig},
            dbg::GrimmDbg,
            remote::channel::send::{GrimmRemoteChannelSend, GrimmRemoteChannelSendWith},
        },
        print::HostPrint,
    },
    nuin::{SignatureParameterTrait, SignatureTrait},
    params::{cache_home, data_home, params, started},
    parse_engine::{LIB_DIRS_CONST, PLUGIN_DIRS_CONST, dirs_value, set_dirs_const},
    plugins::{load_plugin_decls, registry_mtime, registry_path},
    schema::{data_typedef, result_typedef},
};

pub(crate) use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    io::{self, Write},
    ops::ControlFlow,
    os::unix::fs::MetadataExt,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) mod nu {
    pub(crate) use nu_cmd_lang::create_default_context;
    pub(crate) use nu_cmd_extra::add_extra_command_context;
    pub(crate) use nu_cmd_plugin::add_plugin_command_context;
    pub(crate) use nu_command::add_shell_command_context;
    pub(crate) use nu_config::{CliOverrides, EnvAccess, NushellConfigDirs, SystemEnv, resolve_paths};
    pub(crate) use nu_engine::convert_env_values;
    pub(crate) use nu_engine::eval_block;
    pub(crate) use nu_engine::CallExt;
    pub(crate) use nu_engine::command_prelude::Call;
    pub(crate) use nu_parser::parse;
    pub(crate) use nu_plugin_engine::load_plugin_file;
    pub(crate) use nu_protocol::{
        ByteStreamSource, ByteStreamType, Category, CollectionColumns, Config, IntoValue,
        PipelineData, PluginRegistryFile, Example,
        PluginRegistryItemData, Record, ShellError, Signals, Signature, Span, SyntaxShape, Type,
        Value, record, write_all_and_flush,
        ast::{
            Argument, Block, Comparison, Expr, Expression, ExternalArgument, ListItem,
            Operator, Pattern, RecordItem,
        },
        debugger::WithoutDebug,
        engine::{Command, EngineState, Jobs, Stack, StateWorkingSet, ThreadJob},
    };
    pub(crate) use nu_protocol::ast::Call as AstCall;
    pub(crate) use nu_protocol::shell_error::generic::GenericError;
    pub(crate) use nu_std::load_standard_library;
    pub(crate) use nuon::{ToNuonConfig, to_nuon};
}

pub(crate) use nu::{CallExt, IntoValue};

pub(crate) use sourcetrait_common::{cereal, datum};

pub(crate) mod ser {
    pub(crate) use ::serde::Serialize;
}

pub(crate) mod tk {
    pub(crate) use tokio::sync::{
        OwnedSemaphorePermit, Semaphore, oneshot,
        mpsc::{UnboundedSender, unbounded_channel},
    };
}

pub(crate) mod json {
    pub(crate) use serde_json::Value;

    /// A JSON object, as a tool's arguments and schemas carry one.
    pub(crate) type JsonObject = serde_json::Map<String, Value>;
}

pub use crate::{
    cache::{CacheKind, DEFINITION_FILE, cache_dir, definition_file},
    cycle::detect_import_cycle,
    embed::{EVAL_STACK_SIZE, Lane, StatefulEngine, eval_stateless},
    engine::NU_VERSION,
    error::{Diagnostic, EngineError, EngineResult, Severity, Source},
    execute::{ExecuteDef, analyze_definition},
    executor::{Executor, eval_concurrency_cap},
    host::{ChannelSend, EngineHost, RemoteSend},
    lint::{LINT_VIOLATION_CAP, lint_block},
    params::{EngineParams, init},
    parse_engine::{
        LintEngine, ParseEngine, file_start, span_text, span_to_line_col, wrap_as_module,
    },
    plugins::{PluginInfo, ensure_plugin_registry, list_registered_plugins},
    schema::{
        NuTypedef, args_schema_to_signature, check_value, nu_to_args_schema,
        nu_to_result_schema, result_schema_to_signature, typedef_text,
    },
    template::{build_call_source, build_execute_source, json_value_to_nu_value},
};
