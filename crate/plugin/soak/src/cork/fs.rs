use crate::*;

/// Combines a Path and a Nu Span
#[derive(Debug)]
pub struct PathSpan<'a> {
    pub path: &'a Path,
    pub span: nu_protocol::Span,
}
