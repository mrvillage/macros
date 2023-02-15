use std::fmt::Display;

use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};

#[derive(Debug)]
pub struct ParseError {
    pub error: ParseErrorKind,
    pub span: Span,
    pub level: Level,
}

impl ParseError {
    pub(crate) fn new(error: ParseErrorKind, span: Span) -> Self {
        Self {
            error,
            span,
            level: Level::Error,
        }
    }

    pub(crate) fn call_site(error: ParseErrorKind) -> Self {
        Self {
            error,
            span: Span::call_site(),
            level: Level::Error,
        }
    }
}

impl From<ParseError> for Diagnostic {
    fn from(error: ParseError) -> Diagnostic {
        Diagnostic::new(error.level, error.error.into())
    }
}

#[derive(Debug)]
pub enum ParseErrorKind {
    UnknownLiteral(String),
    InvalidByte(u8),
    InvalidEscapeCharacter(u8),
    SuffixNoE,
    InvalidDigit(u8, u8),
    MultipleDecimalPointsInFloat,
    MultipleExponentsInFloat,
    UnexpectedSignInFloat,
    MultipleSignsInFloat,
    MissingExponentDigits,
    MissingUnicodeOpeningBrace,
    TooManyUnicodeDigits,
    MissingUnicodeDigits,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLiteral(literal) => write!(f, "Unknown literal: {literal}"),
            Self::InvalidByte(byte) => write!(f, "Invalid byte with value {byte}"),
            Self::InvalidEscapeCharacter(byte) => {
                write!(f, "Invalid escape character with byte value {byte}")
            },
            Self::SuffixNoE => write!(
                f,
                "The suffix of a numerical literal cannot start with the letter e"
            ),
            Self::InvalidDigit(digit, base) => write!(f, "Invalid digit {digit} for base {base}"),
            Self::MultipleDecimalPointsInFloat => {
                write!(f, "A float literal cannot contain multiple decimal points")
            },
            Self::MultipleExponentsInFloat => {
                write!(f, "A float literal cannot contain multiple exponent parts")
            },
            Self::UnexpectedSignInFloat => {
                write!(
                    f,
                    "A float literal cannot contain a sign in outside the exponent"
                )
            },
            Self::MultipleSignsInFloat => {
                write!(
                    f,
                    "A float literal cannot contain multiple signs in the exponent"
                )
            },
            Self::MissingExponentDigits => {
                write!(
                    f,
                    "The exponent of a float literal must have at least one digit"
                )
            },
            Self::MissingUnicodeOpeningBrace => {
                write!(f, "A unicode escape sequence must start with a {{")
            },
            Self::TooManyUnicodeDigits => {
                write!(f, "A unicode escape sequence must have at most 6 digits")
            },
            Self::MissingUnicodeDigits => {
                write!(f, "A unicode escape sequence must have at least 1 digit")
            },
        }
    }
}

impl From<ParseErrorKind> for String {
    fn from(error: ParseErrorKind) -> Self {
        error.to_string()
    }
}
