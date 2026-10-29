pub(crate) mod capability;
pub(crate) mod value;

pub(crate) use std::fmt;

pub(crate) use nuval::wide;

pub(crate) mod cereal {
    pub(crate) use sourcetrait_cereal_macro::*;
}

pub use crate::{
    capability::Capabilities,
    value::{ValueLimitError, ValueLimitErrorKind, ValueLimits, check_error, check_value},
};

pub use nuval::{
    Bounded, CellPathMemberData, CompileErrorKind, ConfigErrorKind, ConfigWarningKind, DateData,
    GlobData, IoErrorKind, JobErrorKind, NetworkErrorKind, NuVal, NuValError, NuValErrorKind,
    NuValErrorLabel, NuValErrorSource, NuValResult, NuValSeverity, ParseErrorKind,
    ParseWarningKind, RangeData, ShellErrorKind, ShellWarningKind, SpanData, StdIoErrorKind,
    TypedRangeData,
};
