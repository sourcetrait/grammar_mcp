use crate::*;

/// A conversion between nushell and nuval that failed, and why.
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ConvertError {
    /// A value nuval doesn't carry: a closure or a custom value.
    #[snafu(display("a {value_type} value isn't serializable"))]
    Unsupported { value_type: String },
    /// A value that is itself an error, which only a result carries.
    #[snafu(display("an error value: {error}"))]
    ErrorValue { error: Box<nu::ShellError> },
    /// A record naming one column twice.
    #[snafu(display("the column `{column}` appears twice"))]
    DuplicateColumn { column: String },
    /// A range nushell would refuse to make.
    #[snafu(display("an invalid range: {reason}"))]
    Range { reason: String },
    /// A date outside chrono's range, or an offset beyond a day.
    #[snafu(display("an invalid date: {seconds}s and {seconds_fraction}ns at {utc_offset_minus}s"))]
    Date {
        seconds: i64,
        seconds_fraction: u32,
        utc_offset_minus: i32,
    },
}

/// A conversion's result.
pub type ConvertResult<T> = Result<T, ConvertError>;
