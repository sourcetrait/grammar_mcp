use crate::*;

/// A nushell value's serializable result: its value, or its error.
pub type NuValResult = Result<NuVal, NuValError>;

/// The error half of a result, nushell's `Value::Error`.
#[cereal::derived(Data)]
pub enum NuValError {
    Unknown,
}

/// A serializable nushell value: every kind but an error, a closure, a custom.
#[cereal::derived(Data)]
pub enum NuVal {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Glob(GlobData),
    Filesize(i64),
    Duration(i64),
    Date(DateData),
    Range(RangeData),
    Record(Vec<(String, NuVal)>),
    List(Vec<NuVal>),
    Binary(Vec<u8>),
    CellPath(Vec<CellPathMemberData>),
    Nothing,
}

/// A glob pattern, and whether it is taken literally.
#[cereal::derived(Data)]
pub struct GlobData {
    pub glob: String,
    pub no_expand: bool,
}

/// A date and time at a fixed offset, chrono's `DateTime<FixedOffset>`.
#[cereal::derived(Data)]
pub struct DateData {
    pub seconds: u32,
    pub seconds_fraction: u32,
    pub utc_offset_minus: i32,
    pub date_yof: NonZero<i32>,
}

/// A range of ints or of floats.
#[cereal::derived(Data)]
pub enum RangeData {
    Int(TypedRangeData<i64>),
    Float(TypedRangeData<f64>),
}

/// A range's start, its step, and its end.
#[cereal::derived(Data)]
#[serde(bound(
    serialize = "T: cereal::DataCopy",
    deserialize = "T: cereal::DataCopy"
))]
pub struct TypedRangeData<T> {
    pub start: T,
    pub step: T,
    pub end: Bounded<T>,
}

/// A range's end: included, excluded, or none.
#[cereal::derived(Data, Copy)]
#[serde(bound(
    serialize = "T: cereal::DataCopy",
    deserialize = "T: cereal::DataCopy"
))]
pub enum Bounded<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

/// A span's start and end byte offsets.
#[cereal::derived(Data)]
pub struct SpanData {
    pub start: u64,
    pub end: u64,
}

/// An error's code, its message, and its span.
#[cereal::derived(Data)]
pub struct ErrorData {
    pub code: String,
    pub msg: String,
    pub span: Option<SpanData>,
}

/// A cell path member: a column by name or a row by index.
#[cereal::derived(Data)]
pub enum CellPathMemberData {
    String {
        value: String,
        optional: bool,
        case_sensitive: bool,
    },
    Int {
        value: u64,
        optional: bool,
    },
}

impl From<nu::PathMember> for CellPathMemberData {
    fn from(v: nu::PathMember) -> Self {
        match v {
            nu::PathMember::String { val, optional, casing, .. } => Self::String {
                value: val,
                optional,
                case_sensitive: matches!(casing, nu::Casing::Sensitive),
            },
            nu::PathMember::Int { val, optional, .. } => Self::Int {
                value: val as u64,
                optional,
            },
        }
    }
}

/// The span a cell path member takes back in nushell.
const SPAN: nu::Span = nu::Span::unknown();

impl From<CellPathMemberData> for nu::PathMember {
    fn from(v: CellPathMemberData) -> Self {
        match v {
            CellPathMemberData::String { value, optional, case_sensitive } => Self::String {
                val: value,
                optional,
                casing: match case_sensitive { true => nu::Casing::Sensitive, false => nu::Casing::Insensitive },
                span: SPAN, 
            },
            CellPathMemberData::Int { value, optional } => Self::Int {
                val: value as usize,
                span: SPAN,
                optional,
            },
        }
    }
}

impl<T> From<Bound<T>> for Bounded<T> {
    fn from(v: Bound<T>) -> Self {
        match v {
            Bound::Included(v) => Bounded::Included(v),
            Bound::Excluded(v) => Bounded::Excluded(v),
            Bound::Unbounded => Bounded::Unbounded,
        } 
    }
}

impl From<nu_protocol::IntRange> for TypedRangeData<i64> {
    fn from(v: nu_protocol::IntRange) -> Self {
        Self {
            start: v.start(),
            step: v.step(),
            end: v.end().into(),
        }
    }
}

impl From<nu_protocol::FloatRange> for TypedRangeData<f64> {
    fn from(v: nu_protocol::FloatRange) -> Self {
        Self {
            start: v.start(),
            step: v.step(),
            end: v.end().into(),
        }
    }
}

/// A conversion from a nushell value, refused for what nuval can't carry.
pub trait TryFromNu<V: Sized>: Sized {
    fn try_from_nu(value: V) -> Result<Self, ()>;
}

impl TryFromNu<nu::Value> for NuValResult {
    fn try_from_nu(value: nu::Value) -> Result<Self, ()> {
        match value {
            nu::Value::Error { .. } => Ok(Err(NuValError::Unknown)),
            _ => Ok(Ok(NuVal::try_from_nu(value)?))
        }
    }
}

impl TryFromNu<nu::Value> for NuVal {
    fn try_from_nu(value: nu::Value) -> Result<Self, ()> {
        match value {
            nu::Value::Int { val, .. } => Ok(NuVal::Int(val)),
            nu::Value::Float { val, .. } => Ok(NuVal::Float(val)),
            nu::Value::Bool { val, .. } => Ok(NuVal::Bool(val)),
            nu::Value::String { val, .. } => Ok(NuVal::String(val)),
            nu::Value::Range { val, .. } => Ok(NuVal::Range(match *val {
                nu::Range::IntRange(x) => RangeData::Int(
                    TypedRangeData::from(x)
                ),
                nu::Range::FloatRange(x) => RangeData::Float(
                    TypedRangeData::from(x)
                )
            })),
            nu::Value::Record { val, .. } => Ok(NuVal::Record(
                val.into_owned().drain(..)
                    .map(|(k,v)| Self::try_from_nu(v).map(|v| (k, v)))
                    .collect::<Result<Vec<_>, ()>>()?
            )),
            nu::Value::List { vals, .. } => Ok(NuVal::List(
                vals.into_iter()
                    .map(|v| Self::try_from_nu(v))
                    .collect::<Result<Vec<_>, ()>>()?
            )),
            nu::Value::Binary { val, .. } => Ok(NuVal::Binary(val.into_owned())),
            nu::Value::CellPath { val, .. } => Ok(NuVal::CellPath(
                val.members.into_iter()
                    .map(|x| CellPathMemberData::from(x))
                    .collect()
            )),
            nu::Value::Nothing {..} => Ok(NuVal::Nothing),
            _ => Err(()),
        }
    }
}

/*
pub enum Value {
    #[non_exhaustive]
    Bool {
        val: bool,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Int {
        val: i64,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Float {
        val: f64,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    String {
        val: String,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Glob {
        val: String,
        no_expand: bool,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Filesize {
        val: Filesize,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Duration {
        /// The duration in nanoseconds.
        val: i64,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Date {
        val: DateTime<FixedOffset>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Range {
        val: Box<Range>,
        #[serde(skip)]
        signals: Option<Signals>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Record {
        val: SharedCow<Record>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    List {
        vals: Vec<Value>,
        #[serde(skip)]
        signals: Option<Signals>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Closure {
        val: Box<Closure>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Error {
        error: Box<ShellError>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Binary {
        val: Vec<u8>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    CellPath {
        val: CellPath,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Custom {
        val: Box<dyn CustomValue>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    #[non_exhaustive]
    Nothing {
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
}
*/