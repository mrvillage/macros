use proc_macro2::{Spacing, Span};
use quote::quote;

use crate::{tokens::LiteralKind, Delimiter, MacroStream, Pattern, Token};

pub trait Repr {
    fn repr(&self) -> MacroStream;
}

impl Repr for Token {
    fn repr(&self) -> MacroStream {
        match self {
            Self::Group {
                delimiter,
                stream,
                span,
            } => {
                let delimiter = delimiter.repr();
                let stream = stream.repr();
                let span = span.repr();
                quote! {
                    macros_utils::Token::Group {
                        delimiter: #delimiter,
                        stream: #stream,
                        span: #span,
                    }
                }
            },
            Self::Ident { name, span } => {
                let span = span.repr();
                quote! {
                    macros_utils::Token::Ident {
                        name: #name,
                        span: #span,
                    }
                }
            },
            Self::Literal {
                kind,
                value,
                span,
                suffix,
                ..
            } => {
                let kind = kind.repr();
                let span = span.repr();
                quote! {
                    macros_utils::Token::Literal {
                        kind: #kind,
                        value: #value,
                        span: #span,
                        suffix: #suffix,
                        token: None,
                    }
                }
            },
            Self::Punctuation {
                value,
                spacing,
                span,
            } => {
                let spacing = spacing.repr();
                let span = span.repr();
                quote! {
                    macros_utils::Token::Punctuation {
                        value: #value,
                        spacing: #spacing,
                        span: #span,
                    }
                }
            },
        }
        .into()
    }
}

impl Repr for Delimiter {
    fn repr(&self) -> MacroStream {
        match self {
            Delimiter::Brace => quote! { macros_utils::Delimiter::Brace },
            Delimiter::Bracket => quote! { macros_utils::Delimiter::Bracket },
            Delimiter::Parenthesis => quote! { macros_utils::Delimiter::Parenthesis },
            Delimiter::None => quote! { macros_utils::Delimiter::None },
        }
        .into()
    }
}

impl Repr for MacroStream {
    fn repr(&self) -> MacroStream {
        let tokens = self.stream.iter().map(|token| token.repr());
        quote! {
            macros_utils::MacroStream::new(vec![
                #(#tokens),*
            ])
        }
        .into()
    }
}

impl Repr for Span {
    fn repr(&self) -> MacroStream {
        quote! {
            macros_utils::call_site()
        }
        .into()
    }
}

impl Repr for LiteralKind {
    fn repr(&self) -> MacroStream {
        match self {
            Self::Byte => quote! { macros_utils::LiteralKind::Byte },
            Self::Char => quote! { macros_utils::LiteralKind::Char },
            Self::Float => quote! { macros_utils::LiteralKind::Float },
            Self::Integer => quote! { macros_utils::LiteralKind::Integer },
            Self::Str => quote! { macros_utils::LiteralKind::Str },
            Self::StrRaw(h) => quote! { macros_utils::LiteralKind::StrRaw(#h) },
            Self::ByteStr => quote! { macros_utils::LiteralKind::ByteStr },
            Self::ByteStrRaw(h) => quote! { macros_utils::LiteralKind::ByteStrRaw(#h) },
        }
        .into()
    }
}

impl Repr for Spacing {
    fn repr(&self) -> MacroStream {
        match self {
            Self::Alone => quote! { macros_utils::Spacing::Alone },
            Self::Joint => quote! { macros_utils::Spacing::Joint },
        }
        .into()
    }
}

impl Repr for Pattern {
    fn repr(&self) -> MacroStream {
        match self {
            Self::Any => quote! { macros_utils::Pattern::Any },
            Self::Choice(patterns) => {
                let patterns = patterns.repr();
                quote! {
                    macros_utils::Pattern::Choice(#patterns)
                }
            },
            Self::Group(delimiter, pattern) => {
                let delimiter = delimiter.repr();
                let patterns = pattern.repr();
                quote! {
                    macros_utils::Pattern::Group(#delimiter, #patterns)
                }
            },
            Self::OneOrMore(pattern, greedy) => {
                let pattern = pattern.repr();
                quote! {
                    macros_utils::Pattern::OneOrMore(#pattern, #greedy)
                }
            },
            Self::Optional(pattern) => {
                let pattern = pattern.repr();
                quote! {
                    macros_utils::Pattern::Optional(#pattern)
                }
            },
            Self::Parameter(pattern, parameter) => {
                let pattern = pattern.repr();
                quote! {
                    macros_utils::Pattern::Parameter(#pattern, #parameter)
                }
            },
            Self::Token(token) => {
                let token = token.repr();
                quote! {
                    macros_utils::Pattern::Token(#token)
                }
            },
            Self::Validator(stream, _) => {
                let func = match stream {
                    Some(s) => quote! { Some(#s) },
                    None => quote! { None },
                };
                quote! {
                    macros_utils::Pattern::Validator(#stream, #func)
                }
            },
            Self::ZeroOrMore(pattern, greedy) => {
                let pattern = pattern.repr();
                quote! {
                    macros_utils::Pattern::ZeroOrMore(#pattern, #greedy)
                }
            },
        }
        .into()
    }
}

impl<T> Repr for Vec<T>
where
    T: Repr,
{
    fn repr(&self) -> MacroStream {
        let tokens = self.iter().map(|token| token.repr());
        quote! {
            vec![
                #(#tokens),*
            ]
        }
        .into()
    }
}

impl<T> Repr for Option<T>
where
    T: Repr,
{
    fn repr(&self) -> MacroStream {
        match self {
            Some(value) => {
                let value = value.repr();
                quote! {
                    Some(#value)
                }
            },
            None => quote! { None },
        }
        .into()
    }
}

impl<T> Repr for &T
where
    T: Repr,
{
    fn repr(&self) -> MacroStream {
        (*self).repr()
    }
}
