mod pattern;

use macros_utils::{MacroStream, Parse, Token};
use pattern::{pattern_statement, ParserInput};
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
            match next {
                Some(Token::Punctuation { value, .. }) if value == "=>" => {
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
                                let mut #ident = None;
                            }
                        } else {
                            quote! {
                                let mut #ident = vec![];
                            }
                        }
                    });
                    let return_params = raw_params.iter().map(|(ident, _)| {
                        quote! {
                            #ident,
                        }
                    });
                    let struct_fields = raw_params.iter().map(|(ident, optional)| {
                        if *optional {
                            quote! {
                                pub #ident: Option<Vec<macros_utils::MacroStream>>,
                            }
                        } else {
                            quote! {
                                pub #ident: Vec<macros_utils::MacroStream>,
                            }
                        }
                    });
                    let patterns = input.patterns.into_iter().map(|pattern| {
                        let statement = pattern_statement(pattern, &raw_params);
                        quote! {
                            #statement?;
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
                                #(#var_params)*
                                // declare variables for each parameter at the top, if variable is optional it will default to None, otherwise be uninitialized
                                // variables can either be Option<MacroStream> or MacroStream


                                // each pattern needs to be parsed off the stream, and also able to assign to variables

                                // have a match statement for each pattern, if it matches then assign and keep going, if not then error/abort
                                // the match statement (can make it a generic variable above using quote) will return a Result, if it's an error then it will be unwrapped and returned from the function
                                #(#patterns)*
                                Ok(Self {
                                    #(#return_params)*
                                })
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
