pub(crate) mod artifact;
pub(crate) mod capability;
pub(crate) mod channel;
pub(crate) mod codec;
pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod error;
pub(crate) mod limits;
pub(crate) mod operator;
pub(crate) mod peer;
pub(crate) mod runtime;
pub(crate) mod value;

pub(crate) use std::{fmt, io};

pub(crate) use nuval::wide;

pub(crate) use sourcetrait_common::{cereal, datum, subsys};

pub(crate) mod tk {
    pub(crate) use tokio::io::{AsyncRead, AsyncWrite};
}

pub(crate) mod tku {
    pub(crate) use tokio_util::bytes::{Buf, Bytes, BytesMut};
    pub(crate) use tokio_util::codec::{Decoder, Encoder, Framed};
}

pub use crate::{
    artifact::{
        ACTIVE_RESERVATIONS, ARTIFACT_MAX_BYTES, ArtifactDescription, ArtifactId, ArtifactPurpose,
        ArtifactRef, ArtifactReply, ArtifactRequest, ArtifactState, LOG_READ_MAX, LogChunk,
        LogStream, RESERVATION_EXPIRY_SECS, ReadLog, ReserveUpload, SFTP_CHUNK_MAX,
        SFTP_PACKET_MAX, STORE_QUOTA_BYTES, UploadReservation,
    },
    capability::Capabilities,
    channel::{
        ChannelEpoch, ChannelMode, ChannelNotice, ChannelPhase, ChannelSendData, ChannelState,
        EVENT_BYTES_KEY, EVENT_SPILL_DEST, EVENT_SPILL_KEY, HostEvent, MAX_FRAME_BYTES,
        MCP_RESERVED_PREFIX, NOTIFICATION_CAP, REMOTE_RESERVED_PREFIX, RemoteSendData,
        SYNC_QUEUE_CAP, SpamThresholdsData, reserved_prefix,
    },
    codec::{
        BoundedDecode, CODEC_ID, FrameCodec, FrameError, FrameStage, PREFACE, SCHEMA_ID,
        check_codec, decode_payload, framed, payload,
    },
    command::{
        ArgShape, ArgSpec, COMMANDS, CommandKind, CommandSpec, HELP_WORDS, INTERFACE,
        command_by_words, command_spec, render_command_list, render_signatures,
    },
    engine::{
        BUILTINS, CallDoc, CallRequest, CheckSummary, CommitResult, ControlAnswer, ControlRequest,
        Diagnostic, EngineCommand, EngineError, EngineHealth, EngineResult, ExecuteRequest,
        FromEngineSys, Host, HungEval, InspectDoc, LINT_VIOLATION_CAP, Lane, ModuleDoc,
        ProcessRow, RerunRequest, RigDoc, Severity, SignaturesDoc, Source, ToEngineSys,
        ValueAnswer, builtin_equip, builtin_name, display_equip, span_to_line_col,
    },
    error::{LOCAL_HOST, RuntimeError, TransportError},
    limits::{EVENT_FRAME_MAX, HELLO_FRAME_MAX, RPC_FRAME_DEFAULT, RPC_FRAME_HARD, StreamRole},
    operator::{
        BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency, ChannelSpamWarningEmergency,
        CpuWarningEmergency, CriticalEmergency, DiskWarningEmergency, Emergency, EmergencyKind,
        HungEngineThreadEmergency, Observation, RamWarningEmergency, VramWarningEmergency,
    },
    peer::{
        MODEL_CONNECTED, MODEL_DISCONNECTED, MODEL_SENT, MODEL_UNSENT, RemoteChannelEvent,
        RemoteChannelRow, RemoteFailure, RemoteLinkState, RemoteListenResult,
    },
    runtime::{
        ClientInstanceId, ControllerPolicy, InfoReply, LeaseId, OperationId, OperationState,
        OperationStatus, PluginInfo, PurviewRow, PurviewView, RuntimeReply, RuntimeRequest,
        WorkerGeneration,
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
