pub(crate) mod capability;
pub(crate) mod channel;
pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod operator;
pub(crate) mod peer;
pub(crate) mod value;

pub(crate) use std::{fmt, io};

pub(crate) use nuval::wide;

pub(crate) use sourcetrait_common::{cereal, datum, subsys};

pub use crate::{
    capability::Capabilities,
    channel::{
        ChannelEpoch, ChannelMode, ChannelNotice, ChannelPhase, ChannelSendData, ChannelState,
        EVENT_BYTES_KEY, EVENT_SPILL_DEST, EVENT_SPILL_KEY, HostEvent, MAX_FRAME_BYTES,
        MCP_RESERVED_PREFIX, NOTIFICATION_CAP, REMOTE_RESERVED_PREFIX, RemoteSendData,
        SYNC_QUEUE_CAP, SpamThresholdsData, reserved_prefix,
    },
    command::{
        ArgShape, ArgSpec, COMMANDS, CommandKind, CommandSpec, HELP_WORDS, INTERFACE,
        command_by_words, command_spec, render_command_list, render_signatures,
    },
    engine::{
        BUILTINS, CallDoc, CallRequest, CheckSummary, CommitResult, ControlAnswer, ControlRequest,
        Diagnostic, EngineCommand, EngineError, EngineHealth, EngineResult, ExecuteRequest,
        FromEngineSys, Host, HungEval, InspectDoc, Lane, ModuleDoc, ProcessRow, RerunRequest,
        RigDoc, Severity, SignaturesDoc, Source, ToEngineSys, ValueAnswer, builtin_equip,
        builtin_name, display_equip,
    },
    operator::{
        BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency, ChannelSpamWarningEmergency,
        CpuWarningEmergency, CriticalEmergency, DiskWarningEmergency, Emergency, EmergencyKind,
        HungEngineThreadEmergency, Observation, RamWarningEmergency, VramWarningEmergency,
    },
    peer::{
        MODEL_CONNECTED, MODEL_DISCONNECTED, MODEL_SENT, MODEL_UNSENT, RemoteChannelEvent,
        RemoteChannelRow, RemoteFailure, RemoteLinkState, RemoteListenResult,
    },
    value::{ValueLimitError, ValueLimitErrorKind, ValueLimits, check_error, check_value},
};

pub use nuval::{
    Bounded, CellPathMemberData, CompileErrorKind, ConfigErrorKind, ConfigWarningKind, DateData,
    GlobData, IoErrorKind, JobErrorKind, NetworkErrorKind, NuVal, NuValError, NuValErrorKind,
    NuValErrorLabel, NuValErrorSource, NuValResult, NuValSeverity, ParseErrorKind,
    ParseWarningKind, RangeData, ShellErrorKind, ShellWarningKind, SpanData, StdIoErrorKind,
    TypedRangeData,
};
