pub(crate) mod capability;
pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod operator;
pub(crate) mod value;

pub(crate) use std::{fmt, io};

pub(crate) use nuval::wide;

pub(crate) use sourcetrait_common::{cereal, datum, subsys};

pub use crate::{
    capability::Capabilities,
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
    value::{ValueLimitError, ValueLimitErrorKind, ValueLimits, check_error, check_value},
};

pub use nuval::{
    Bounded, CellPathMemberData, CompileErrorKind, ConfigErrorKind, ConfigWarningKind, DateData,
    GlobData, IoErrorKind, JobErrorKind, NetworkErrorKind, NuVal, NuValError, NuValErrorKind,
    NuValErrorLabel, NuValErrorSource, NuValResult, NuValSeverity, ParseErrorKind,
    ParseWarningKind, RangeData, ShellErrorKind, ShellWarningKind, SpanData, StdIoErrorKind,
    TypedRangeData,
};
