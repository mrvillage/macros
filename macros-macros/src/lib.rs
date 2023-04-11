use std::str::FromStr;

use macros_utils::{
    call_site, MacroStream, Match, Parse, ParserInput, ParserOutput, Repr, Spacing, Token,
};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::quote;

/// Create a parser based on a set of patterns.
///
/// See `Pattern` for more information on the available patterns.
///
/// # Example
/// ```rs
/// use macros_core::parser;
///
/// parser! {
///     NameOfParserAndOutputStruct => {}$ { {}$ : param }@
/// }
///
/// let output: NameOfParserAndOutputStruct = NameOfParserAndOutputStruct::parse(
///     &mut proc_macro2::TokenStream::from_str("hi hello")
///         .unwrap()
///         .into(),
/// );
#[proc_macro_error]
#[proc_macro]
pub fn parser(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match MacroStream::from_tokens(stream.into()) {
        Err(err) => err.into_diagnostic().abort(),
        Ok(stream) => parser_impl(stream).into(),
    }
}

#[derive(Clone)]
struct Empty {}

impl ParserOutput for Empty {
    fn set_match(&mut self, _: &str, _: Match) {}
    fn name() -> &'static str {
        "Empty"
    }
}

fn parser_impl(mut stream: MacroStream) -> TokenStream {
    let name = stream.pop();
    match name {
        Some(Token::Ident { name, .. }) => {
            let next = stream.pop();
            let next2 = stream.pop();
            match (next, next2) {
                (
                    Some(Token::Punctuation {
                        value: '=',
                        spacing: Spacing::Joint,
                        ..
                    }),
                    Some(Token::Punctuation {
                        value: '>',
                        spacing: Spacing::Alone,
                        ..
                    }),
                ) => {
                    let input = match ParserInput::<Empty>::parse(&mut stream) {
                        Err(err) => err.into_diagnostic().abort(),
                        Ok(input) => input,
                    };
                    let patterns = &input
                        .patterns
                        .iter()
                        .map(|p| p.repr(&name))
                        .collect::<Vec<_>>();
                    let struct_name = Token::Ident {
                        name: name.clone(),
                        span: Span::call_site(),
                    };
                    let raw_params = input
                        .params()
                        .into_iter()
                        .map(|(name, optional)| {
                            let ident = Token::Ident {
                                name,
                                span: Span::call_site(),
                            };
                            (ident, optional)
                        })
                        .collect::<Vec<_>>();
                    let var_params = raw_params.iter().map(|(ident, optional)| {
                        if *optional {
                            quote! {
                                #ident: None,
                            }
                        } else {
                            quote! {
                                #ident: macros_utils::Match::None,
                            }
                        }
                    });
                    let struct_fields = raw_params.iter().map(|(ident, optional)| {
                        if *optional {
                            quote! {
                                pub #ident: Option<macros_utils::Match>,
                            }
                        } else {
                            quote! {
                                pub #ident: macros_utils::Match,
                            }
                        }
                    });
                    let patterns_const = Token::Ident {
                        name: format!("__{}_PATTERNS", name.to_ascii_uppercase()),
                        span: call_site(),
                    };
                    let set_params = raw_params.iter().map(|(ident, _)| {
                        let name = ident.ident().unwrap();
                        quote! {
                            #name => self.#ident = value,
                        }
                    });
                    quote! {
                        #[derive(Debug, Clone)]
                        pub struct #struct_name {
                            #(#struct_fields)*
                        }

                        macros_utils::lazy_static! {
                            static ref #patterns_const: Vec<macros_utils::Pattern<#struct_name>> = vec![
                                #(#patterns,)*
                            ];
                        }

                        #[allow(clippy::never_loop)]
                        impl macros_utils::Parse for #struct_name {
                            fn parse(stream: &mut macros_utils::MacroStream) -> Result<Self, macros_utils::MacrosError> {
                                let mut o = Self {
                                    #(#var_params)*
                                };
                                let (res, o) = macros_utils::Pattern::<#struct_name>::match_patterns(std::borrow::Cow::Owned(o), &#patterns_const, stream);
                                match res {
                                    Ok(_) => Ok(o.into_owned()),
                                    Err(e) => Err(e),
                                }
                            }
                        }

                        impl macros_utils::ParserOutput for #struct_name {
                            fn set_match(&mut self, name: &str, value: macros_utils::Match) {
                                match name {
                                    #(#set_params)*
                                    _ => (),
                                }
                            }

                            fn name() -> &'static str {
                                #name
                            }
                        }
                    }
                },
                _ => abort_call_site!("expected => after the name of the parser"),
            }
        },
        _ => abort_call_site!("expected the name of the parser first"),
    }
}
