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
    pub seconds: i64,
    pub seconds_fraction: u32,
    pub utc_offset_minus: i32,
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

impl CellPathMemberData {
    /// The member as nushell's, at a span.
    pub fn into_nu(self, span: nu::Span) -> nu::PathMember {
        match self {
            Self::String { value, optional, case_sensitive } => nu::PathMember::String {
                val: value,
                span,
                optional,
                casing: if case_sensitive {
                    nu::Casing::Sensitive
                } else {
                    nu::Casing::Insensitive
                },
            },
            Self::Int { value, optional } => nu::PathMember::Int {
                val: value as usize,
                span,
                optional,
            },
        }
    }
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
        v.into_nu(SPAN)
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

impl<T> From<Bounded<T>> for Bound<T> {
    fn from(v: Bounded<T>) -> Self {
        match v {
            Bounded::Included(v) => Bound::Included(v),
            Bounded::Excluded(v) => Bound::Excluded(v),
            Bounded::Unbounded => Bound::Unbounded,
        }
    }
}

impl From<nu::IntRange> for TypedRangeData<i64> {
    fn from(v: nu::IntRange) -> Self {
        Self {
            start: v.start(),
            step: v.step(),
            end: v.end().into(),
        }
    }
}

impl From<nu::FloatRange> for TypedRangeData<f64> {
    fn from(v: nu::FloatRange) -> Self {
        Self {
            start: v.start(),
            step: v.step(),
            end: v.end().into(),
        }
    }
}

impl From<nu::Range> for RangeData {
    fn from(v: nu::Range) -> Self {
        match v {
            nu::Range::IntRange(x) => Self::Int(x.into()),
            nu::Range::FloatRange(x) => Self::Float(x.into()),
        }
    }
}

impl TryFrom<RangeData> for nu::Range {
    type Error = ConvertError;

    fn try_from(v: RangeData) -> ConvertResult<Self> {
        match v {
            RangeData::Int(r) => int_range(r).map(nu::Range::IntRange),
            RangeData::Float(r) => float_range(r).map(nu::Range::FloatRange),
        }
    }
}

/// An int range from its parts, refused where nushell's constructor refuses.
fn int_range(r: TypedRangeData<i64>) -> ConvertResult<nu::IntRange> {
    if r.step == 0 {
        return RangeSnafu { reason: "its step is zero" }.fail();
    }
    let next = r.start.checked_add(r.step).context(RangeSnafu {
        reason: "its second value overflows",
    })?;
    if let Bounded::Included(end) | Bounded::Excluded(end) = r.end
        && (next < r.start) != (end < r.start)
    {
        return RangeSnafu { reason: "its step runs away from its end" }.fail();
    }
    range_from_parts(r.start, r.step, Bound::from(r.end))
}

/// A float range from its parts, refused where nushell's constructor refuses.
fn float_range(r: TypedRangeData<f64>) -> ConvertResult<nu::FloatRange> {
    if !r.start.is_finite() {
        return RangeSnafu { reason: "its start isn't finite" }.fail();
    }
    let next = r.start + r.step;
    if !next.is_finite() || next == r.start {
        return RangeSnafu { reason: "its step is zero or not finite" }.fail();
    }
    let end = match r.end {
        Bounded::Included(end) | Bounded::Excluded(end) if end.is_nan() => {
            return RangeSnafu { reason: "its end isn't a number" }.fail();
        }
        Bounded::Included(end) | Bounded::Excluded(end) if end.is_infinite() => Bound::Unbounded,
        bounded => Bound::from(bounded),
    };
    if let Bound::Included(end) | Bound::Excluded(end) = end
        && (next < r.start) != (end < r.start)
    {
        return RangeSnafu { reason: "its step runs away from its end" }.fail();
    }
    range_from_parts(r.start, r.step, end)
}

/// A nushell range built exactly from its start, its step, and its end.
fn range_from_parts<T, R>(start: T, step: T, end: Bound<T>) -> ConvertResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let parts = serde_json::json!({ "start": start, "step": step, "end": end });
    serde_json::from_value(parts).map_err(|e| ConvertError::Range {
        reason: e.to_string(),
    })
}

impl From<chrono::DateTime<chrono::FixedOffset>> for DateData {
    fn from(v: chrono::DateTime<chrono::FixedOffset>) -> Self {
        Self {
            seconds: v.timestamp(),
            seconds_fraction: v.timestamp_subsec_nanos(),
            utc_offset_minus: v.offset().local_minus_utc(),
        }
    }
}

impl TryFrom<DateData> for chrono::DateTime<chrono::FixedOffset> {
    type Error = ConvertError;

    fn try_from(v: DateData) -> ConvertResult<Self> {
        let DateData {
            seconds,
            seconds_fraction,
            utc_offset_minus,
        } = v;
        let utc = chrono::DateTime::from_timestamp(seconds, seconds_fraction);
        let offset = chrono::FixedOffset::east_opt(utc_offset_minus);
        match (utc, offset) {
            (Some(utc), Some(offset)) => Ok(utc.with_timezone(&offset)),
            _ => DateSnafu {
                seconds,
                seconds_fraction,
                utc_offset_minus,
            }
            .fail(),
        }
    }
}

/// A conversion from a nushell value, refused for what nuval can't carry.
pub trait TryFromNu<V: Sized>: Sized {
    fn try_from_nu(value: V) -> ConvertResult<Self>;
}

/// A conversion back to a nushell value, every part at one span.
pub trait TryIntoNu {
    fn try_into_nu(self, span: nu::Span) -> ConvertResult<nu::Value>;
}

impl TryFromNu<nu::Value> for NuValResult {
    fn try_from_nu(value: nu::Value) -> ConvertResult<Self> {
        match NuVal::try_from_nu(value) {
            Ok(val) => Ok(Ok(val)),
            Err(ConvertError::ErrorValue { .. }) => Ok(Err(NuValError::Unknown)),
            Err(e) => Err(e),
        }
    }
}

impl TryFromNu<nu::Value> for NuVal {
    fn try_from_nu(value: nu::Value) -> ConvertResult<Self> {
        Ok(match value {
            nu::Value::Bool { val, .. } => NuVal::Bool(val),
            nu::Value::Int { val, .. } => NuVal::Int(val),
            nu::Value::Float { val, .. } => NuVal::Float(val),
            nu::Value::String { val, .. } => NuVal::String(val),
            nu::Value::Glob { val, no_expand, .. } => NuVal::Glob(GlobData {
                glob: val,
                no_expand,
            }),
            nu::Value::Filesize { val, .. } => NuVal::Filesize(val.get()),
            nu::Value::Duration { val, .. } => NuVal::Duration(val),
            nu::Value::Date { val, .. } => NuVal::Date(val.into()),
            nu::Value::Range { val, .. } => NuVal::Range((*val).into()),
            nu::Value::Record { val, .. } => NuVal::Record(
                val.into_owned()
                    .into_iter()
                    .map(|(k, v)| Self::try_from_nu(v).map(|v| (k, v)))
                    .collect::<ConvertResult<Vec<_>>>()?,
            ),
            nu::Value::List { vals, .. } => NuVal::List(
                vals.into_iter()
                    .map(Self::try_from_nu)
                    .collect::<ConvertResult<Vec<_>>>()?,
            ),
            nu::Value::Binary { val, .. } => NuVal::Binary(val.into_owned()),
            nu::Value::CellPath { val, .. } => NuVal::CellPath(
                val.members
                    .into_iter()
                    .map(CellPathMemberData::from)
                    .collect(),
            ),
            nu::Value::Nothing { .. } => NuVal::Nothing,
            nu::Value::Error { error, .. } => return ErrorValueSnafu { error }.fail(),
            nu::Value::Closure { .. } => {
                return UnsupportedSnafu { value_type: "closure" }.fail();
            }
            nu::Value::Custom { val, .. } => {
                return UnsupportedSnafu {
                    value_type: val.type_name(),
                }
                .fail();
            }
        })
    }
}

impl TryIntoNu for NuVal {
    fn try_into_nu(self, span: nu::Span) -> ConvertResult<nu::Value> {
        Ok(match self {
            NuVal::Bool(v) => nu::Value::bool(v, span),
            NuVal::Int(v) => nu::Value::int(v, span),
            NuVal::Float(v) => nu::Value::float(v, span),
            NuVal::String(v) => nu::Value::string(v, span),
            NuVal::Glob(GlobData { glob, no_expand }) => nu::Value::glob(glob, no_expand, span),
            NuVal::Filesize(v) => nu::Value::filesize(nu::Filesize::new(v), span),
            NuVal::Duration(v) => nu::Value::duration(v, span),
            NuVal::Date(v) => nu::Value::date(v.try_into()?, span),
            NuVal::Range(v) => nu::Value::range(v.try_into()?, span),
            NuVal::Record(pairs) => {
                let mut record = nu::Record::with_capacity(pairs.len());
                for (column, v) in pairs {
                    if record.contains(&column) {
                        return DuplicateColumnSnafu { column }.fail();
                    }
                    record.push(column, v.try_into_nu(span)?);
                }
                nu::Value::record(record, span)
            }
            NuVal::List(vals) => nu::Value::list(
                vals.into_iter()
                    .map(|v| v.try_into_nu(span))
                    .collect::<ConvertResult<Vec<_>>>()?,
                span,
            ),
            NuVal::Binary(v) => nu::Value::binary(v, span),
            NuVal::CellPath(members) => nu::Value::cell_path(
                nu::CellPath {
                    members: members.into_iter().map(|m| m.into_nu(span)).collect(),
                },
                span,
            ),
            NuVal::Nothing => nu::Value::nothing(span),
        })
    }
}
