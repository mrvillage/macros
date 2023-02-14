use std::fmt::Display;

use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};

#[derive(Debug)]
pub struct MacrosError {
    pub error: MacrosErrorKind,
    pub span: Span,
    pub level: Level,
}

impl MacrosError {
    pub(crate) fn new(error: MacrosErrorKind, span: Span) -> Self {
        Self {
            error,
            span,
            level: Level::Error,
        }
    }

    pub(crate) fn call_site(error: MacrosErrorKind) -> Self {
        Self {
            error,
            span: Span::call_site(),
            level: Level::Error,
        }
    }

    pub(crate) fn warning(mut self) -> Self {
        self.level = Level::Warning;
        self
    }
}

impl From<MacrosError> for Diagnostic {
    fn from(error: MacrosError) -> Diagnostic {
        Diagnostic::new(error.level, error.error.into())
    }
}

#[derive(Debug)]
pub enum MacrosErrorKind {
    // ExpectedToken(Token),
    ExpectedIdent(String),
    ExpectedArbitraryIdent,
    // ExpectedDelimiter(proc_macro2::Delimiter),
    // ExpectedPunctuation(proc_macro2::Punctuation),
    // ExpectedLiteral(Literal),
}

impl Display for MacrosErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Self::ExpectedToken(token) => write!(f, "Expected token: {}", token),
            Self::ExpectedIdent(ident) => write!(f, "Expected ident: {}", ident),
            // Self::ExpectedDelimiter(delimiter) => write!(f, "Expected delimiter: {}", delimiter),
            // Self::ExpectedPunctuation(punctuation) => write!(f, "Expected punctuation: {}", punctuation),
            // Self::ExpectedLiteral(literal) => write!(f, "Expected literal: {}", literal),
            Self::ExpectedArbitraryIdent => write!(f, "Expected arbitrary ident"),
        }
    }
}

impl From<MacrosErrorKind> for String {
    fn from(error: MacrosErrorKind) -> Self {
        error.to_string()
    }
}
