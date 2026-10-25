use crate::*;

pub type CorkResult<T> = Result<T, CorkError>;

#[derive(Debug, snafu::Snafu)]
pub enum CorkError {
    #[snafu(display("{op}: {path}",
        path = Color::LightPurple.paint(path.display().to_string())))]
    PathIO {
        source: io::Error,
        path: PathBuf,
        op: PathOp,
        span: Option<nu_protocol::Span>,
    },
    #[snafu(display("{kind} not found: {path}",
        path = Color::LightPurple.paint(path.display().to_string())))]
    PathNotFound {
        kind: PathKind,
        path: PathBuf,
        span: Option<nu_protocol::Span>,
    },
    #[snafu(display("Failed to copy {kind}: {from} >> {to}",
        from = Color::LightPurple.paint(from.display().to_string()),
        to = Color::LightPurple.paint(to.display().to_string())
    ))]
    PathCopy {
        source: io::Error,
        kind: PathKind,
        from: PathBuf,
        to: PathBuf,
        span: Option<nu_protocol::Span>,
    },
    #[snafu(display("Expected UTF8: {s}",
        s = Color::LightPurple.paint(s.display().to_string())
    ))]
    Utf8 {
        s: OsString,
        span: Option<nu_protocol::Span>,
    },
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, snafu::Snafu)]
pub enum PathKind {
    #[default]
    #[snafu(display("Path"))]
    Path,
    #[snafu(display("Directory"))]
    Dir,
    #[snafu(display("File"))]
    File,
}

impl PathKind {
    pub fn not_found_kind(&self) -> nu_protocol::shell_error::io::ErrorKind {
        match self {
            Self::Path | Self::File => nu_protocol::shell_error::io::ErrorKind::FileNotFound,
            Self::Dir => nu_protocol::shell_error::io::ErrorKind::DirectoryNotFound,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, snafu::Snafu)]
pub enum PathOp {
    #[snafu(display("Failed to create directory"))]
    CreateDir,
    #[snafu(display("Failed to read directory attributes"))]
    DirMeta,
    #[snafu(display("Failed to list directory"))]
    ListDir,
    #[snafu(display("Failed to write to file"))]
    WriteFile,
    #[snafu(display("Failed to read file"))]
    ReadFile,
}

impl ErrorTrait for CorkError {
    fn meta(&self) -> Option<ErrorMeta> {
        const UTF8: Option<ErrorMeta> = ErrorMeta::label("nu::cork::utf8", "non-utf8");
        
        match self {
            Self::PathIO{..} => None,
            Self::PathNotFound{..} => None,
            Self::PathCopy{..} => None,
            Self::Utf8{..} => UTF8,
        }
    }

    fn into_span(self) -> Option<nu_protocol::Span> {
        match self {
            Self::PathNotFound { span,.. } => span,
            Self::PathCopy { span,.. } => span,
            Self::PathIO { span,.. } => span,
            Self::Utf8 { span,.. } => span,
        }
    }
    
    fn nu(self, span: nu_protocol::Span) -> nu_protocol::LabeledError {
        let msg = self.to_string();
        match self {
            Self::PathNotFound { path, kind, span: path_span } => {
                let mut e = nu_protocol::LabeledError::from_diagnostic(
                    &nu_protocol::ShellError::Io(nu_protocol::shell_error::io::IoError::new(
                        kind.not_found_kind(),
                        path_span.unwrap_or(span),
                        Some(path),
                    ))
                );
                e.msg = msg;
                e.help = None;
                e
            },
            Self::PathCopy { source, from, span: path_span, .. } => {
                let mut e = nu_protocol::LabeledError::from_diagnostic(
                    &nu_protocol::ShellError::Io(nu_protocol::shell_error::io::IoError::new(
                        nu_protocol::shell_error::io::ErrorKind::from_std(source.kind()),
                        path_span.unwrap_or(span),
                        Some(from),
                    ))
                );
                e.msg = msg;
                e.help = None;
                e
            },
            Self::PathIO { source, path, span: path_span, .. } => {
                let mut e = nu_protocol::LabeledError::from_diagnostic(
                    &nu_protocol::ShellError::Io(nu_protocol::shell_error::io::IoError::new(
                        nu_protocol::shell_error::io::ErrorKind::from_std(source.kind()),
                        path_span.unwrap_or(span),
                        Some(path),
                    ))
                );
                e.msg = msg;
                e.help = None;
                e
            },
            e @ Self::Utf8{..} => Self::default_nu(e, span),
        }
    }
}

impl CorkError {
    pub fn default_nu(e: impl ErrorTrait, span: nu_protocol::Span) -> nu_protocol::LabeledError {
        let msg = e.to_string();
        let meta = e.meta().unwrap_or_default();
        let span = e.into_span().unwrap_or(span);
        let mut e = nu_protocol::LabeledError::new(msg)
            .with_code(meta.code);
        if let Some(label) = meta.label {
            e = e.with_label(label, span);
        }
        
        e
    }

    pub fn path_io(source: io::Error, path: impl Into<PathBuf>, op: PathOp, span: impl Into<Option<nu_protocol::Span>>) -> Self {
        Self::PathIO { source, path: path.into(), op, span: span.into() }
    }
    
    pub fn path_copy(source: io::Error, kind: PathKind, from: impl Into<PathBuf>, to: impl Into<PathBuf>, span: impl Into<Option<nu_protocol::Span>>) -> Self {
        Self::PathCopy { source, from: from.into(), to: to.into(), kind, span: span.into() }
    }

    pub fn utf8(s: impl Into<OsString>, span: impl Into<Option<nu_protocol::Span>>) -> Self {
        Self::Utf8 { s: s.into(), span: span.into() }
    }

    pub fn err_path_not_found<T>(path: impl Into<PathBuf>, kind: PathKind, span: impl Into<Option<nu_protocol::Span>>) -> CorkResult<T> {
        Err(Self::PathNotFound { path: path.into(), kind, span: span.into() })
    }
}

#[derive(Debug)]
pub struct ErrorMeta {
    pub code: &'static str,
    pub label: Option<&'static str>,
}

impl ErrorMeta {
    pub const DEFAULT_CODE: &str = "nu::shell::error";
    pub const DEFAULT_LABEL: &str = "here";
    pub const DEFAULT: Self = Self {
        code: Self::DEFAULT_CODE,
        label: Some(Self::DEFAULT_LABEL),
    };
    
    pub const fn label(code: &'static str, label: &'static str) -> Option<Self> {
        Some(Self { code, label: Some(label) })
    }
    
    pub const fn code(code: &'static str) -> Option<Self> {
        Some(Self { code, label: None })
    }
}

impl Default for ErrorMeta {
    #[inline] fn default() -> Self { Self::DEFAULT }
}

pub trait ErrorTrait: Sized + Display {
    /// Some for anything not handled manually in [Self::nu]
    fn meta(&self) -> Option<ErrorMeta>;
    fn into_span(self) -> Option<nu_protocol::Span>;
    
    fn nu(self, span: nu_protocol::Span) -> nu_protocol::LabeledError {
        CorkError::default_nu(self, span)
    }
}