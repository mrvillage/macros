use proc_macro2::{Spacing, Span};
use quote::quote;

use crate::{tokens::LiteralKind, Delimiter, MacroStream, ParserOutput, Pattern, Token};

pub trait Repr {
    fn repr(&self, name: &str) -> MacroStream;
}

impl Repr for Token {
    fn repr(&self, name: &str) -> MacroStream {
        match self {
            Self::Group {
                delimiter,
                stream,
                span,
            } => {
                let delimiter = delimiter.repr(name);
                let stream = stream.repr(name);
                let span = span.repr(name);
                quote! {
                    macros_utils::Token::Group {
                        delimiter: #delimiter,
                        stream: #stream,
                        span: #span,
                    }
                }
            },
            Self::Ident { name: n, span } => {
                let span = span.repr(name);
                quote! {
                    macros_utils::Token::Ident {
                        name: #n.to_string(),
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
                let kind = kind.repr(name);
                let span = span.repr(name);
                quote! {
                    macros_utils::Token::Literal {
                        kind: #kind,
                        value: #value.to_string(),
                        span: #span,
                        suffix: #suffix.to_string(),
                        token: None,
                    }
                }
            },
            Self::Punctuation {
                value,
                spacing,
                span,
            } => {
                let spacing = spacing.repr(name);
                let span = span.repr(name);
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
    fn repr(&self, _: &str) -> MacroStream {
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
    fn repr(&self, name: &str) -> MacroStream {
        let tokens = self.stream.iter().map(|token| token.repr(name));
        quote! {
            macros_utils::MacroStream::new(vec![
                #(#tokens),*
            ])
        }
        .into()
    }
}

impl Repr for Span {
    fn repr(&self, _: &str) -> MacroStream {
        quote! {
            macros_utils::call_site()
        }
        .into()
    }
}

impl Repr for LiteralKind {
    fn repr(&self, _: &str) -> MacroStream {
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
    fn repr(&self, _: &str) -> MacroStream {
        match self {
            Self::Alone => quote! { macros_utils::Spacing::Alone },
            Self::Joint => quote! { macros_utils::Spacing::Joint },
        }
        .into()
    }
}

impl<T> Repr for Pattern<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    fn repr(&self, name: &str) -> MacroStream {
        let type_name = Token::Ident {
            name: name.to_string(),
            span: Span::call_site(),
        };
        match self {
            Self::Any => quote! { macros_utils::Pattern::<#type_name>::Any },
            Self::Choice(patterns) => {
                let patterns = patterns.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::Choice(#patterns)
                }
            },
            Self::Group(delimiter, pattern) => {
                let delimiter = delimiter.repr(name);
                let patterns = pattern.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::Group(#delimiter, #patterns)
                }
            },
            Self::OneOrMore(pattern, greedy) => {
                let pattern = pattern.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::OneOrMore(#pattern, #greedy)
                }
            },
            Self::Optional(pattern) => {
                let pattern = pattern.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::Optional(#pattern)
                }
            },
            Self::Parameter(pattern, parameter) => {
                let pattern = pattern.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::Parameter(#pattern, #parameter)
                }
            },
            Self::Token(token) => {
                let token = token.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::Token(#token)
                }
            },
            Self::Validator(stream, _) => {
                let func = match stream {
                    Some(s) => quote! { Some(#s) },
                    None => quote! { None },
                };
                quote! {
                    macros_utils::Pattern::<#type_name>::Validator(#stream, #func)
                }
            },
            Self::ZeroOrMore(pattern, greedy) => {
                let pattern = pattern.repr(name);
                quote! {
                    macros_utils::Pattern::<#type_name>::ZeroOrMore(#pattern, #greedy)
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
    fn repr(&self, name: &str) -> MacroStream {
        let tokens = self.iter().map(|token| token.repr(name));
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
    fn repr(&self, name: &str) -> MacroStream {
        match self {
            Some(value) => {
                let value = value.repr(name);
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
    fn repr(&self, name: &str) -> MacroStream {
        (*self).repr(name)
    }
}
