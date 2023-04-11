use std::{
    collections::VecDeque,
    fmt::{Debug, Display, Formatter},
    str::FromStr,
};

use proc_macro2::{Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};

use crate::{
    parsers::{
        get_byte_at, parse_lit_byte, parse_lit_byte_str, parse_lit_byte_str_raw, parse_lit_char,
        parse_lit_float, parse_lit_int, parse_lit_str, parse_lit_str_raw,
    },
    MacroStream, ParseError, ParseErrorKind, ParseResult,
};

/// The delimiter of a group of tokens
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delimiter {
    /// `( ... )`
    Parenthesis,
    /// `{ ... }`
    Brace,
    /// `[ ... ]`
    Bracket,
    /// `Ø ... Ø`
    /// An invisible delimiter around something like $var
    /// Ensures that if $var is substituted in as 1 + 2
    /// order of operations is preserved
    None,
}

impl Display for Delimiter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parenthesis => write!(f, "parenthesis"),
            Self::Brace => write!(f, "braces"),
            Self::Bracket => write!(f, "brackets"),
            Self::None => write!(f, "none"),
        }
    }
}

impl From<Delimiter> for proc_macro2::Delimiter {
    fn from(delimiter: Delimiter) -> Self {
        match delimiter {
            Delimiter::Parenthesis => Self::Parenthesis,
            Delimiter::Brace => Self::Brace,
            Delimiter::Bracket => Self::Bracket,
            Delimiter::None => Self::None,
        }
    }
}
impl From<&Delimiter> for proc_macro2::Delimiter {
    fn from(delimiter: &Delimiter) -> Self {
        match delimiter {
            Delimiter::Parenthesis => Self::Parenthesis,
            Delimiter::Brace => Self::Brace,
            Delimiter::Bracket => Self::Bracket,
            Delimiter::None => Self::None,
        }
    }
}

impl From<proc_macro2::Delimiter> for Delimiter {
    fn from(delimiter: proc_macro2::Delimiter) -> Self {
        match delimiter {
            proc_macro2::Delimiter::Parenthesis => Self::Parenthesis,
            proc_macro2::Delimiter::Brace => Self::Brace,
            proc_macro2::Delimiter::Bracket => Self::Bracket,
            proc_macro2::Delimiter::None => Self::None,
        }
    }
}
impl From<&proc_macro2::Delimiter> for Delimiter {
    fn from(delimiter: &proc_macro2::Delimiter) -> Self {
        match delimiter {
            proc_macro2::Delimiter::Parenthesis => Self::Parenthesis,
            proc_macro2::Delimiter::Brace => Self::Brace,
            proc_macro2::Delimiter::Bracket => Self::Bracket,
            proc_macro2::Delimiter::None => Self::None,
        }
    }
}

/// A token in a macro.
#[derive(Clone, Debug)]
pub enum Token {
    Ident {
        name: String,
        span: Span,
    },

    Group {
        delimiter: Delimiter,
        stream: MacroStream,
        span: Span,
    },

    Literal {
        kind: LiteralKind,
        value: String,
        span: Span,
        suffix: String,
        token: Option<Literal>,
    },

    /// either a single character for something like `+`
    /// or a longer string for something like `+=` or `+===`
    Punctuation {
        value: char,
        spacing: Spacing,
        span: Span,
    },
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Ident { name, .. },
                Self::Ident {
                    name: other_name, ..
                },
            ) => name == other_name,
            (
                Self::Group {
                    delimiter, stream, ..
                },
                Self::Group {
                    delimiter: other_delimiter,
                    stream: other_stream,
                    ..
                },
            ) => delimiter == other_delimiter && stream == other_stream,
            (
                Self::Literal {
                    kind,
                    value,
                    suffix,
                    ..
                },
                Self::Literal {
                    kind: other_kind,
                    value: other_value,
                    suffix: other_suffix,
                    ..
                },
            ) => kind == other_kind && value == other_value && suffix == other_suffix,
            (
                Self::Punctuation { value, .. },
                Self::Punctuation {
                    value: other_value, ..
                },
            ) => value == other_value,
            _ => false,
        }
    }
}

impl Eq for Token {}

/// The kind of literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiteralKind {
    Byte,
    Char,
    Integer,
    Float,
    Str,
    // the u8 is the number of `#` symbols used in the raw string
    StrRaw(u8),
    ByteStr,
    // the u8 is the number of `#` symbols used in the raw string
    ByteStrRaw(u8),
}

impl LiteralKind {
    fn to_ident(&self) -> Token {
        Token::Ident {
            name: match self {
                Self::Byte => "Byte",
                Self::Char => "Char",
                Self::Integer => "Integer",
                Self::Float => "Float",
                Self::Str => "String",
                Self::StrRaw(_) => "StrRaw",
                Self::ByteStr => "ByteStr",
                Self::ByteStrRaw(_) => "ByteStrRaw",
            }
            .to_string(),
            span: Span::call_site(),
        }
    }
}

impl Token {
    pub fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::Group { .. } => quote!(),
            Self::Ident { name, .. } => {
                quote! {
                    macros_utils::Token::Ident {
                        name: #name.to_string(),
                        span: macros_utils::call_site(),
                    }
                }
            },
            Self::Literal {
                kind,
                suffix,
                value,
                ..
            } => {
                let kind = kind.to_ident();
                quote! {
                    macros_utils::Token::Literal {
                        kind: macros_utils::LiteralKind::#kind,
                        value: #value.to_string(),
                        span: macros_utils::call_site(),
                        suffix: #suffix.to_string(),
                        token: None,
                    }
                }
            },
            Self::Punctuation { value, .. } => {
                quote! {
                    macros_utils::Token::Punctuation {
                        value: #value.to_string(),
                        span: macros_utils::call_site(),
                    }
                }
            },
        }
    }

    pub fn from_tokens(queue: &mut VecDeque<TokenTree>) -> ParseResult<Self> {
        let token = queue.pop_front().unwrap();
        Ok(match token {
            TokenTree::Ident(ident) => Self::Ident {
                name: ident.to_string(),
                span: ident.span(),
            },
            TokenTree::Group(group) => Self::Group {
                delimiter: group.delimiter().into(),
                stream: MacroStream::from_tokens(group.stream())?,
                span: group.span(),
            },
            TokenTree::Literal(lit) => {
                let literal = lit.to_string();
                match get_byte_at(&literal, 0) {
                    b'"' => {
                        let (value, suffix) = parse_lit_str(&literal)?;
                        Self::Literal {
                            kind: LiteralKind::Str,
                            value,
                            span: lit.span(),
                            suffix,
                            token: Some(lit),
                        }
                    },
                    b'r' => {
                        let (value, suffix, hashtags) = parse_lit_str_raw(&literal)?;
                        Self::Literal {
                            kind: LiteralKind::StrRaw(hashtags),
                            value,
                            span: lit.span(),
                            suffix,
                            token: Some(lit),
                        }
                    },
                    b'b' => match get_byte_at(&literal, 1) {
                        b'"' => {
                            let (value, suffix) = parse_lit_byte_str(&literal)?;
                            Self::Literal {
                                kind: LiteralKind::ByteStr,
                                value,
                                span: lit.span(),
                                suffix,
                                token: Some(lit),
                            }
                        },
                        b'r' => {
                            let (value, suffix, hashtags) = parse_lit_byte_str_raw(&literal)?;
                            Self::Literal {
                                kind: LiteralKind::ByteStrRaw(hashtags),
                                value,
                                span: lit.span(),
                                suffix,
                                token: Some(lit),
                            }
                        },
                        b'\'' => {
                            let (value, suffix) = parse_lit_byte(&literal)?;
                            Self::Literal {
                                kind: LiteralKind::Byte,
                                value,
                                span: lit.span(),
                                suffix,
                                token: Some(lit),
                            }
                        },
                        _ => {
                            return Err(ParseError::new(
                                lit.span(),
                                ParseErrorKind::UnknownLiteral(literal),
                            ))
                        },
                    },
                    b'\'' => {
                        let (value, suffix) = parse_lit_char(&literal)?;
                        Self::Literal {
                            kind: LiteralKind::Char,
                            value,
                            span: lit.span(),
                            suffix,
                            token: Some(lit),
                        }
                    },
                    b'0'..=b'9' | b'-' => {
                        if let Some((value, suffix)) = parse_lit_float(&literal)? {
                            Self::Literal {
                                kind: LiteralKind::Float,
                                value,
                                span: lit.span(),
                                suffix,
                                token: Some(lit),
                            }
                        } else {
                            let (value, suffix) = parse_lit_int(&literal)?;
                            Self::Literal {
                                kind: LiteralKind::Integer,
                                value,
                                span: lit.span(),
                                suffix,
                                token: Some(lit),
                            }
                        }
                    },
                    _ => {
                        return Err(ParseError::new(
                            lit.span(),
                            ParseErrorKind::UnknownLiteral(literal),
                        ))
                    },
                }
            },
            TokenTree::Punct(p) => Self::Punctuation {
                value: p.as_char(),
                spacing: p.spacing(),
                span: p.span(),
            },
        })
    }

    pub fn ident(&self) -> Option<&str> {
        if let Token::Ident { name, .. } = self {
            Some(name)
        } else {
            None
        }
    }

    pub fn group(&self) -> Option<&MacroStream> {
        if let Token::Group { stream, .. } = self {
            Some(stream)
        } else {
            None
        }
    }

    pub fn lit_suffix(&self) -> Option<&str> {
        if let Token::Literal { suffix, .. } = self {
            Some(suffix)
        } else {
            None
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Token::Ident { span, .. } => *span,
            Token::Group { span, .. } => *span,
            Token::Literal { span, .. } => *span,
            Token::Punctuation { span, .. } => *span,
        }
    }

    pub fn punctuation(&self) -> Option<&char> {
        if let Token::Punctuation { value, .. } = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn lit_byte(&self) -> Option<u8> {
        if let Token::Literal {
            kind: LiteralKind::Byte,
            value,
            ..
        } = self
        {
            if let Ok(value) = value.parse::<u8>() {
                return Some(value);
            }
        }
        None
    }

    pub fn lit_char(&self) -> Option<char> {
        if let Token::Literal {
            kind: LiteralKind::Char,
            value,
            ..
        } = self
        {
            if let Ok(value) = value.parse::<char>() {
                return Some(value);
            }
        }
        None
    }

    pub fn lit_integer<I>(&self) -> Option<I>
    where
        I: FromStr,
    {
        if let Token::Literal {
            kind: LiteralKind::Integer,
            value,
            ..
        } = self
        {
            if let Ok(value) = value.parse::<I>() {
                return Some(value);
            }
        }
        None
    }

    pub fn lit_float<F>(&self) -> Option<F>
    where
        F: FromStr,
    {
        if let Token::Literal {
            kind: LiteralKind::Float,
            value,
            ..
        } = self
        {
            if let Ok(value) = value.parse::<F>() {
                return Some(value);
            }
        }
        None
    }

    pub fn lit_str(&self) -> Option<&str> {
        if let Token::Literal {
            kind: LiteralKind::Str,
            value,
            ..
        } = self
        {
            Some(value)
        } else {
            None
        }
    }

    pub fn lit_str_raw(&self) -> Option<&str> {
        if let Token::Literal {
            kind: LiteralKind::StrRaw(_),
            value,
            ..
        } = self
        {
            Some(value)
        } else {
            None
        }
    }

    pub fn lit_byte_str(&self) -> Option<&[u8]> {
        if let Token::Literal {
            kind: LiteralKind::ByteStr,
            value,
            ..
        } = self
        {
            Some(value.as_bytes())
        } else {
            None
        }
    }

    pub fn lit_byte_str_raw(&self) -> Option<&[u8]> {
        if let Token::Literal {
            kind: LiteralKind::ByteStrRaw(_),
            value,
            ..
        } = self
        {
            Some(value.as_bytes())
        } else {
            None
        }
    }
}

/// Note: Converting a Literal will result in the loss of the suffix and typically also specific information regarding what type it is, the value itself will not be lost (large u128 numbers exceeding 127 bits may lose their last bit though).
impl ToTokens for Token {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append::<TokenTree>(match self {
            Self::Group {
                delimiter,
                stream,
                span,
            } => {
                let mut token = Group::new(delimiter.into(), stream.to_token_stream());
                token.set_span(*span);
                token.into()
            },
            Self::Ident { name, span } => Ident::new(name, *span).into(),
            Self::Literal {
                kind,
                value,
                token,
                span,
                ..
            } => match token {
                Some(lit) => lit.clone().into(),
                None => {
                    let mut token = match kind {
                        LiteralKind::Byte => Literal::u8_unsuffixed(value.parse::<u8>().unwrap()),
                        LiteralKind::ByteStr => Literal::byte_string(value.as_bytes()),
                        LiteralKind::ByteStrRaw(_) => Literal::byte_string(value.as_bytes()),
                        LiteralKind::Char => Literal::character(value.parse::<char>().unwrap()),
                        LiteralKind::Float => {
                            Literal::f64_unsuffixed(value.parse::<f64>().unwrap())
                        },
                        LiteralKind::Integer => {
                            Literal::i128_unsuffixed(value.parse::<i128>().unwrap())
                        },
                        LiteralKind::Str => Literal::string(value),
                        LiteralKind::StrRaw(_) => Literal::string(value),
                    };
                    token.set_span(*span);
                    token.into()
                },
            },
            Self::Punctuation {
                value,
                span,
                spacing,
            } => {
                let mut token = Punct::new(*value, *spacing);
                token.set_span(*span);
                token.into()
            },
        });
    }
}
