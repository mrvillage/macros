use std::{error::Error, fmt::Display};

use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};
use thiserror::Error;

use crate::{Delimiter, Token};

/// The error type for this crate. Can be either a `Parse(ParseError)` from this crate or a `User(Box<dyn Error + Send + Sync>)` user error.
#[derive(Debug, Error)]
pub enum MacrosError {
    #[error(transparent)]
    Parse(ParseError),
    #[error(transparent)]
    User(Box<dyn Error + Send + Sync>),
}

impl From<Box<dyn Error + Send + Sync>> for MacrosError {
    fn from(error: Box<dyn Error + Send + Sync>) -> Self {
        Self::User(error)
    }
}

impl From<ParseError> for MacrosError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl MacrosError {
    /// Convert the error into a `proc_macro_error::Diagnostic`.
    pub fn into_diagnostic(self) -> Diagnostic {
        match self {
            Self::Parse(error) => error.into_diagnostic(),
            Self::User(error) => Diagnostic::new(Level::Error, error.to_string()),
        }
    }

    /// Add a message to the error if it is a `Self::Parse` and the error is an `UnexpectedEndOfInput`.
    pub fn unexpected_end_of_input(mut self, msg: &str) -> Self {
        if let Self::Parse(error) = &mut self {
            error.unexpected_end_of_input(msg);
        };
        self
    }
}

/// A parse error encountered while parsing a `MacroStream`.
#[derive(Debug, Error)]
pub struct ParseError {
    #[source]
    pub error: ParseErrorKind,
    pub span: Span,
    pub level: Level,
}

impl ParseError {
    /// Create a new parse error with the given span and kind.
    pub fn new(span: Span, error: ParseErrorKind) -> Self {
        Self {
            error,
            span,
            level: Level::Error,
        }
    }

    /// Create a new parse error with the given kind and the `Span::call_site()` span.
    pub fn call_site(error: ParseErrorKind) -> Self {
        Self {
            error,
            span: Span::call_site(),
            level: Level::Error,
        }
    }

    /// Convert the error into a `proc_macro_error::Diagnostic`.
    pub fn into_diagnostic(self) -> Diagnostic {
        Diagnostic::spanned(self.span, self.level, self.error.to_string())
    }

    /// Add a message to the error if it is an `UnexpectedEndOfInput` error.
    pub fn unexpected_end_of_input(&mut self, msg: &str) {
        if let ParseErrorKind::UnexpectedEndOfInput(s) = &mut self.error {
            s.push_str(msg);
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl From<ParseError> for Diagnostic {
    fn from(error: ParseError) -> Diagnostic {
        error.into_diagnostic()
    }
}

/// The specific kind of parse error encountered.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ParseErrorKind {
    #[error("Unknown literal: {0}")]
    UnknownLiteral(String),
    #[error("Invalid byte with value {0}")]
    InvalidByte(u8),
    #[error("Invalid escape character with byte value {0}")]
    InvalidEscapeCharacter(u8),
    #[error("The suffix of a numerical literal cannot start with the letter e")]
    SuffixNoE,
    #[error("Invalid digit {0} for base {1}")]
    InvalidDigit(u8, u8),
    #[error("A float literal cannot contain multiple decimal points")]
    MultipleDecimalPointsInFloat,
    #[error("A float literal cannot contain multiple exponent parts")]
    MultipleExponentsInFloat,
    #[error("A float literal cannot contain a sign in outside the exponent")]
    UnexpectedSignInFloat,
    #[error("A float literal cannot contain multiple signs in the exponent")]
    MultipleSignsInFloat,
    #[error("The exponent of a float literal must have at least one digit")]
    MissingExponentDigits,
    #[error("A unicode escape sequence must start with a {{")]
    MissingUnicodeOpeningBrace,
    #[error("A unicode escape sequence must end with a }}")]
    TooManyUnicodeDigits,
    #[error("A unicode escape sequence must have at least one digit")]
    MissingUnicodeDigits,
    #[error("Unexpected end of input, message: {0}")]
    UnexpectedEndOfInput(String),
    #[error("Expected {0:?}, but found {1:?}")]
    Expected(Token, Token),
    #[error("No matching choice found")]
    NoMatchingChoice,
    #[error("Expected a group delimited by {0}")]
    ExpectedGroup(Delimiter),
    #[error("Input is too long")]
    InputTooLong,
    #[error("Expected one or more repetitions, but found none")]
    ExpectedRepetition,
    #[error("Validator must have a non-validator pattern preceding it")]
    InvalidValidatorPosition,
    #[error("Validator failed with message: {0}")]
    ValidatorFailed(String),
}
