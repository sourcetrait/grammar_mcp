#[cfg(feature = "nu")]
pub(crate) mod error;
#[cfg(feature = "nu")]
pub(crate) mod json;
pub(crate) mod kind {
    pub(crate) mod compile;
    pub(crate) mod config;
    pub(crate) mod io;
    pub(crate) mod job;
    pub(crate) mod network;
    pub(crate) mod parse;
    pub(crate) mod shell;
}
pub(crate) mod val;
pub(crate) mod val_error;
pub(crate) mod width;
#[cfg(all(test, feature = "nu"))]
mod tests {
    mod val;
    mod val_error;
}

#[cfg(feature = "nu")]
pub(crate) use crate::error::*;

#[cfg(feature = "nu")]
pub(crate) use snafu::OptionExt;

#[cfg(feature = "nu")]
pub(crate) use std::ops::Bound;

#[cfg(feature = "nu")]
pub(crate) mod nu {
    pub(crate) use nu_protocol::{
        ChainedError, CompileError, ConfigError, ConfigWarning, ErrorLabel, Filesize, FloatRange,
        IntRange, LabeledError, ParseError, ParseWarning, Range, Record, ShellError, ShellWarning,
        Span, Value,
        ast::{CellPath, PathMember},
        casing::Casing,
        shell_error::{
            ErrorSite,
            io::ErrorKind as IoKind,
            job::JobError,
            network::{DnsErrorKind, NetworkError},
        },
    };
}

pub(crate) use sourcetrait_common::cereal;

pub use crate::{
    kind::{
        compile::CompileErrorKind,
        config::{ConfigErrorKind, ConfigWarningKind},
        io::{IoErrorKind, StdIoErrorKind},
        job::JobErrorKind,
        network::NetworkErrorKind,
        parse::{ParseErrorKind, ParseWarningKind},
        shell::{ShellErrorKind, ShellWarningKind},
    },
    val::{
        Bounded, CellPathMemberData, DateData, GlobData, NuVal, NuValResult, RangeData, SpanData,
        TypedRangeData,
    },
    val_error::{NuValError, NuValErrorKind, NuValErrorLabel, NuValErrorSource, NuValSeverity},
    width::wide,
};

#[cfg(feature = "nu")]
pub use crate::{
    error::{ConvertError, ConvertResult},
    json::json_value_to_nu_value,
    val::{TryFromNu, TryIntoNu},
};
