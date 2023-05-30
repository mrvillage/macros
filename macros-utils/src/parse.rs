use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use crate::{LiteralKind, MacroStream, MacrosError, ParseError, ParseErrorKind, Token};

/// Parse a `MacroStream` into a `Self`.
///
/// # Example
/// ```rs
/// use macros_utils::{Parse, MacroStream};
///
/// #[derive(Debug, Clone)]
/// struct MyStruct {
///     pub a: Token,
///     pub b: Token,
/// }
///
/// impl Parse for MyStruct {
///     fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
///         let a = input.pop_or_err()?;
///         let b = input.pop_or_err()?;
///         Ok(Self { a, b })
///     }
/// }
pub trait Parse: Sized {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError>;
}

impl Parse for String {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
        let token = input.pop_or_err()?;
        match token {
            Token::Literal {
                kind: LiteralKind::Str,
                value,
                ..
            } => Ok(value),
            _ => Err(MacrosError::Parse(ParseError::new(
                token.span(),
                ParseErrorKind::User("expected str".into()),
            ))),
        }
    }
}

fn parse_int<T>(input: &mut MacroStream) -> Result<T, MacrosError>
where
    T: FromStr<Err = ParseIntError>,
{
    let token = input.pop_or_err()?;
    match token {
        Token::Literal {
            kind: LiteralKind::Integer,
            ref value,
            ..
        } => match value.parse() {
            Ok(v) => Ok(v),
            Err(e) => Err(token.to_parse_error(e.to_string().into()).into()),
        },
        _ => Err(token.to_parse_error("expected float".into()).into()),
    }
}

macro_rules! impl_parse_int {
    ($($ty:ty),*) => {
        $(
            impl Parse for $ty {
                fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
                    parse_int(input)
                }
            }
        )*
    };
    () => {

    };
}

impl_parse_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

fn parse_float<T>(input: &mut MacroStream) -> Result<T, MacrosError>
where
    T: FromStr<Err = ParseFloatError>,
{
    let token = input.pop_or_err()?;
    match token {
        Token::Literal {
            kind: LiteralKind::Float,
            ref value,
            ..
        } => match value.parse() {
            Ok(v) => Ok(v),
            Err(e) => Err(token.to_parse_error(e.to_string().into()).into()),
        },
        _ => Err(token.to_parse_error("expected float".into()).into()),
    }
}

macro_rules! impl_parse_float {
    ($($ty:ty),*) => {
        $(
            impl Parse for $ty {
                fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
                    parse_float(input)
                }
            }
        )*
    };
    () => {

    };
}

impl_parse_float!(f32, f64);

impl Parse for bool {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
        let token = input.pop_or_err()?;
        match token {
            Token::Ident { name, .. } if name == "true" => Ok(true),
            Token::Ident { name, .. } if name == "false" => Ok(false),
            _ => Err(token.to_parse_error("expected bool".into()).into()),
        }
    }
}

impl Parse for char {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
        let token = input.pop_or_err()?;
        match token {
            Token::Literal {
                kind: LiteralKind::Char,
                value,
                ..
            } => Ok(value.chars().next().unwrap()),
            _ => Err(token.to_parse_error("expected char".into()).into()),
        }
    }
}
