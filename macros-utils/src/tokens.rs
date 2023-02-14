use std::{collections::VecDeque, fmt::Debug, str::FromStr};

use proc_macro2::{Spacing, Span, TokenTree};

use crate::{
    parsers::{
        get_byte_at, parse_lit_byte, parse_lit_byte_str, parse_lit_byte_str_raw, parse_lit_char,
        parse_lit_float, parse_lit_int, parse_lit_str, parse_lit_str_raw,
    },
    MacroStream,
};

/// The delimiter of a group of tokens
#[derive(Clone, Copy, Debug)]
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

#[derive(Debug)]
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
    },

    /// either a single character for something like `+`
    /// or a longer string for something like `+=` or `+===`
    Punctuation {
        value: String,
        span: Span,
    },
}

#[derive(Debug)]
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

impl Token {
    pub fn from_tokens(queue: &mut VecDeque<TokenTree>) -> Self {
        let token = queue.pop_front().unwrap();
        match token {
            TokenTree::Ident(ident) => Self::Ident {
                name: ident.to_string(),
                span: ident.span(),
            },
            TokenTree::Group(group) => Self::Group {
                delimiter: group.delimiter().into(),
                stream: group.stream().into(),
                span: group.span(),
            },
            TokenTree::Literal(lit) => {
                let literal = lit.to_string();
                match get_byte_at(&literal, 0) {
                    b'"' => {
                        let (value, suffix) = parse_lit_str(&literal);
                        Self::Literal {
                            kind: LiteralKind::Str,
                            value,
                            span: lit.span(),
                            suffix,
                        }
                    },
                    b'r' => {
                        let (value, suffix, hashtags) = parse_lit_str_raw(&literal);
                        Self::Literal {
                            kind: LiteralKind::StrRaw(hashtags),
                            value,
                            span: lit.span(),
                            suffix,
                        }
                    },
                    b'b' => match get_byte_at(&literal, 1) {
                        b'"' => {
                            let (value, suffix) = parse_lit_byte_str(&literal);
                            Self::Literal {
                                kind: LiteralKind::ByteStr,
                                value,
                                span: lit.span(),
                                suffix,
                            }
                        },
                        b'r' => {
                            let (value, suffix, hashtags) = parse_lit_byte_str_raw(&literal);
                            Self::Literal {
                                kind: LiteralKind::ByteStrRaw(hashtags),
                                value,
                                span: lit.span(),
                                suffix,
                            }
                        },
                        b'\'' => {
                            let (value, suffix) = parse_lit_byte(&literal);
                            Self::Literal {
                                kind: LiteralKind::Byte,
                                value,
                                span: lit.span(),
                                suffix,
                            }
                        },
                        _ => {
                            panic!("unknown literal: {}", literal)
                        },
                    },
                    b'\'' => {
                        let (value, suffix) = parse_lit_char(&literal);
                        Self::Literal {
                            kind: LiteralKind::Char,
                            value,
                            span: lit.span(),
                            suffix,
                        }
                    },
                    b'0'..=b'9' | b'-' => {
                        if let Some((value, suffix)) = parse_lit_float(&literal) {
                            Self::Literal {
                                kind: LiteralKind::Float,
                                value,
                                span: lit.span(),
                                suffix,
                            }
                        } else {
                            let (value, suffix) = parse_lit_int(&literal);
                            Self::Literal {
                                kind: LiteralKind::Integer,
                                value,
                                span: lit.span(),
                                suffix,
                            }
                        }
                    },
                    _ => {
                        panic!("unknown literal: {}", literal)
                    },
                }
            },
            TokenTree::Punct(punct) => {
                queue.push_front(TokenTree::Punct(punct));
                let mut punct = String::new();
                let mut span: Option<Span> = None;
                loop {
                    let token = queue.pop_front();
                    if let Some(TokenTree::Punct(p)) = &token {
                        punct.push_str(&p.to_string());
                        if let Spacing::Alone = p.spacing() {
                            break;
                        }
                        if let Some(s) = span {
                            let new = s.join(p.span());
                            if let Some(new) = new {
                                span = Some(new);
                            } else {
                                span = Some(p.span());
                            }
                        } else {
                            span = Some(p.span());
                        }
                    }
                    if let Some(token) = token {
                        queue.push_front(token);
                        break;
                    }
                }
                Self::Punctuation {
                    value: punct,
                    span: span.unwrap_or_else(Span::call_site),
                }
            },
        }
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

    pub fn punctuation(&self) -> Option<&str> {
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
