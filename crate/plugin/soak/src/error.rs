use crate::*;

pub(crate) type SoakResult<T> = Result<T, SoakError>;

#[derive(Debug, snafu::Snafu)]
pub(crate) enum SoakError {
    Cork {
        source: CorkError,
    },
    #[snafu(display("Liquid parsing failed: {source}"))]
    LiquidParse {
        source: liquid::Error,
    },
    #[snafu(display("Cannot convert to Liquid value: {invalid}"))]
    LiquidValue {
        invalid: NuType,
        span: nu_protocol::Span,
    }
}

#[derive(Debug, snafu::Snafu)]
pub(crate) enum NuType {
    Binary, // non-utf8
    Custom, // lacks base value
    Closure,
    Date, // non-standard
    Error,
}

impl ErrorTrait for SoakError {
    fn meta(&self) -> Option<ErrorMeta> {
        match self {
            Self::Cork { source } => source.meta(),
            Self::LiquidParse { .. } => ErrorMeta::code("nu::soak::liquid::parse"),
            Self::LiquidValue { invalid, .. } => {
                let label = match invalid {
                    NuType::Binary => "non-utf8",
                    NuType::Date => "non-standard",
                    NuType::Custom => "lacks base value",
                    _ => "incompatible",
                };
                
                ErrorMeta::label("nu::soak::value", label)
            },
        }
    }

    fn into_span(self) -> Option<nu_protocol::Span> {
        match self {
            Self::Cork { source } => source.into_span(),
            Self::LiquidValue { span,.. } => Some(span),
            Self::LiquidParse { .. } => None,
        }
    }
}

impl From<CorkError> for SoakError {
    fn from(source: CorkError) -> Self { Self::Cork { source } }
}