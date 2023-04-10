use macros_utils::{pattern_statement, MacroStream, Parse, ParserInput, Spacing, Token};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::quote;

#[proc_macro_error]
#[proc_macro]
pub fn parser(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match MacroStream::from_tokens(stream.into()) {
        Err(err) => err.into_diagnostic().abort(),
        Ok(stream) => parser_impl(stream).into(),
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
                    let input = match ParserInput::parse(&mut stream) {
                        Err(err) => err.into_diagnostic().abort(),
                        Ok(input) => input,
                    };
                    let struct_name = Token::Ident {
                        name,
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
                    let patterns = input.patterns.into_iter().map(|pattern| {
                        let statement = pattern_statement(pattern, &raw_params);
                        quote! {
                            #statement?;
                        }
                    });
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

                        #[allow(clippy::never_loop)]
                        impl macros_utils::Parse for #struct_name {
                            fn parse(stream: &mut macros_utils::MacroStream) -> Result<Self, macros_utils::MacrosError> {
                                let mut self = Self {
                                    #(#var_params)*
                                };
                                // declare variables for each parameter at the top, if variable is optional it will default to None, otherwise be uninitialized
                                // variables can either be Option<MacroStream> or MacroStream


                                // each pattern needs to be parsed off the stream, and also able to assign to variables

                                // have a match statement for each pattern, if it matches then assign and keep going, if not then error/abort
                                // the match statement (can make it a generic variable above using quote) will return a Result, if it's an error then it will be unwrapped and returned from the function
                                #(#patterns)*
                                Ok(self)
                            }
                        }

                        impl macros_utils::SetMatch for #struct_name {
                            fn set_match(&mut self, name: &str, value: macros_utils::Match) {
                                match name {
                                    #(#set_params)*
                                    _ => (),
                                }
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
