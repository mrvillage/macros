use macros_utils::{Delimiter, MacroStream, MacrosError, Parse, Spacing, Token};
use proc_macro2::TokenStream;
use proc_macro_error::{abort, abort_call_site};
use quote::quote;

#[derive(Debug)]
pub struct ParserInput {
    pub patterns: Vec<Pattern>,
}

/// {...}? indicates optional
/// {... :name}@ indicates a parameter
/// {...}* indicates zero or more
/// {...}+ indicates one or more
/// {... | ... | ...}& indicates a choice
/// ... indicates a token to match exactly
/// {{...}} escapes the {} grouping
/// To escape any of the special endings, use ~whatever before the ending, to escape the tilde use ~~
/// {}$ indicates an arbitrary token, if used in a zero or more or one or more then it will consume the stream until the next pattern matches (non-greedy)
/// {}$$ indicates an arbitrary token, if used in a zero or more or one or more then it will consume the remainder of the stream (greedy)
/// {...}= indicates a validation function, should be anything of type type `Fn(&DataStruct, &Vec<Token>) -> Result<(), String>` as it will be interpolated directly into the code expecting that type
#[derive(Debug)]
pub enum Pattern {
    Optional(Vec<Pattern>),
    Parameter(Vec<Pattern>, String),
    ZeroOrMore(Vec<Pattern>),
    OneOrMore(Vec<Pattern>),
    Choice(Vec<Vec<Pattern>>),
    Token(Token),
    Group(Delimiter, Vec<Pattern>),
    Any(bool),
    Validator(MacroStream),
}

impl ParserInput {
    pub fn params(&self) -> Vec<(String, bool)> {
        let mut params = vec![];
        for pattern in &self.patterns {
            params.extend(pattern.params());
        }
        params
    }
}

impl Parse for ParserInput {
    fn parse(stream: &mut MacroStream) -> Result<Self, MacrosError> {
        Ok(Self {
            patterns: stream_to_patterns(stream)?,
        })
    }
}

fn stream_to_patterns(stream: &mut MacroStream) -> Result<Vec<Pattern>, MacrosError> {
    let mut patterns = Vec::new();
    while !stream.is_empty() {
        patterns.push(Pattern::parse(stream)?);
    }
    Ok(patterns)
}

impl Parse for Pattern {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
        let token = input.pop_or_err().map_err(|mut e| {
            e.unexpected_end_of_input("started parsing a pattern and found no start");
            e
        })?;
        Ok(match token {
            Token::Group {
                delimiter: Delimiter::Brace,
                mut stream,
                ..
            } => {
                let token = stream.pop_or_err();
                let ending = input.peek();
                match token {
                    Ok(Token::Group {
                        delimiter: Delimiter::Brace,
                        stream: mut inner_stream,
                        ..
                    }) if stream.is_empty() => {
                        Self::Group(Delimiter::Brace, stream_to_patterns(&mut inner_stream)?)
                    },
                    Ok(token) => {
                        let t = match ending {
                            Some(Token::Punctuation { value: '?', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                Self::Optional(stream_to_patterns(&mut stream)?)
                            },
                            Some(Token::Punctuation { value: '*', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                Self::ZeroOrMore(stream_to_patterns(&mut stream)?)
                            },
                            Some(Token::Punctuation { value: '+', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                Self::OneOrMore(stream_to_patterns(&mut stream)?)
                            },
                            Some(Token::Punctuation { value: '@', spacing: Spacing::Alone, .. }) => {
                                let mut span = token.span();
                                stream.push_front(token);
                                let mut patterns = vec![];
                                while !stream.is_empty() {
                                    let token = stream.peek();
                                    if let Some(token) = token.as_ref() {
                                        span = token.span();
                                    }
                                    match token {
                                        Some(Token::Punctuation { value: ':', spacing: Spacing::Alone, .. }) => {
                                            stream.pop();
                                            break;
                                        },
                                        _ => patterns.push(Pattern::parse(&mut stream)?),
                                    }
                                }
                                if stream.is_empty() {
                                    abort!(span, "expected a pattern, a colon, then an ident, (like some_pattern_here:name), found end of input");
                                }
                                if patterns.is_empty() {
                                    abort!(span, "expected a pattern, a colon, then an ident, (like some_pattern_here:name), found no pattern");
                                }
                                let token = stream.pop_or_err()?;
                                match token {
                                    Token::Ident { name, .. } => Self::Parameter(patterns, name),
                                    _ => abort!(token.span(), "expected an identifier"),
                                }
                            },
                            Some(Token::Punctuation { value: '&', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                let mut patterns = vec![];
                                let mut current = vec![];
                                while !stream.is_empty() {
                                    let token = stream.peek();
                                    match token {
                                        Some(Token::Punctuation { value: '|', spacing: Spacing::Alone, .. }) => {
                                            if !current.is_empty() {
                                                patterns.push(current);
                                                current = vec![];
                                            }
                                            stream.pop();
                                            continue;
                                        },
                                        _ => current.push(Pattern::parse(&mut stream)?),
                                    }
                                }
                                if !current.is_empty() {
                                    patterns.push(current);
                                }
                                Self::Choice(patterns)
                            },
                            Some(Token::Punctuation { value: '$', spacing: Spacing::Alone, .. }) => {
                                Self::Any(false)
                            },
                            Some(Token::Punctuation { value: '$', spacing: Spacing::Joint, .. }) => {
                                Self::Any(match input.peek_at(1) {
                                    Some(Token::Punctuation { value: '$', spacing: Spacing::Alone, .. }) => {
                                        input.pop(); // pops the previous token off so that this one is popped off at the end of the match
                                        true
                                    },
                                    _ => false
                                })
                            },
                            Some(Token::Punctuation { value: '=', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                Self::Validator(stream)
                            },
                            _ => {
                                abort!(token.span(), "expected one of ?*+=~@&$ after single braces")
                            },
                        };
                        input.pop();
                        t
                    },
                    Err(_) => match ending {
                        Some(Token::Punctuation { value: '$', spacing: Spacing::Alone, .. }) => {
                            input.pop();
                            Self::Any(false)
                        },
                        Some(Token::Punctuation { value: '$', spacing: Spacing::Joint, .. }) => {
                            input.pop();
                            Self::Any(match input.peek_at(1) {
                                Some(Token::Punctuation { value: '$', spacing: Spacing::Alone, .. }) => {
                                    input.pop();
                                    true
                                },
                                _ => false
                            })
                        },
                        _ => abort_call_site!("found empty group, expected either an any pattern (like {}$) after the braces or something in the braces"),
                    },
                }
            },
            Token::Punctuation { value: '~', .. } => {
                let next = input.pop_or_err().map_err(|mut e| {
                    e.unexpected_end_of_input("started parsing a pattern and found no start");
                    e
                })?;
                match next {
                    next @ Token::Punctuation {
                        value: '?' | '*' | '+' | '=' | '~' | '@' | '&' | '$',
                        ..
                    } => Self::Token(next),
                    _ => abort!(next.span(), "expected one of ?*+=~@&$ after tilde"),
                }
            },
            Token::Group {
                delimiter,
                mut stream,
                ..
            } => Self::Group(delimiter, stream_to_patterns(&mut stream)?),
            token => Self::Token(token),
        })
    }
}

impl Pattern {
    pub fn params(&self) -> Vec<(String, bool)> {
        let mut params = vec![];
        match self {
            Self::Group(_, patterns) => {
                for i in patterns {
                    params.extend(i.params());
                }
            },
            Self::Optional(patterns) => {
                for i in patterns {
                    params.extend(i.params().into_iter().map(|(name, _)| (name, true)));
                }
            },
            Self::ZeroOrMore(patterns) => {
                for i in patterns {
                    params.extend(i.params());
                }
            },
            Self::OneOrMore(patterns) => {
                for i in patterns {
                    params.extend(i.params());
                }
            },
            Self::Choice(patterns) => {
                for i in patterns {
                    for j in i {
                        params.extend(j.params());
                    }
                }
            },
            Self::Parameter(patterns, name) => {
                for i in patterns {
                    params.extend(i.params());
                }
                params.push((name.clone(), false));
            },
            _ => {},
        };
        params
    }
}

pub fn pattern_statement(pattern: Pattern, params: &Vec<(Token, bool)>) -> TokenStream {
    // should assume it has a stream variable that it is MEANT to modify and pop values off of on either success or failure
    // the caller is responsible for ensuring that, on error, the stream is returned to the state it was in before the pattern was attempted (should only happen in the case of Optional which will fork the stream and deal with it itself)
    match pattern {
        // TODO: reimplement this and make it work with non-greedy matching as
        Pattern::Any(_) => {
            quote! {
                {
                    let token = stream.pop_or_err();
                    if let Err(e) = token {
                        Err(macros_utils::MacrosError::Parse(e))
                    } else {
                        Ok(macros_utils::Match::One(token.unwrap()))
                    }
                }
            }
        },
        Pattern::Choice(choices) => {
            let choices = choices.into_iter().map(|choice| {
                let choice = choice
                    .into_iter()
                    .map(|p| pattern_statement(p, params))
                    .map(|statement| {
                        quote! {
                            let r: Result<macros_utils::Match, macros_utils::MacrosError> = #statement;
                            if let Err(r) = r {
                                break Err(r);
                            }
                            matches.push(r.unwrap());
                        }
                    });
                quote! {
                    let r: Result<macros_utils::Match, macros_utils::MacrosError> = loop {
                        let mut matches = vec![];
                        let p = {
                            let mut stream = stream.fork();
                            #(#choice)*
                            stream.popped()
                        };
                        // stream variable is now the original stream, need to pop off the tokens
                        stream.popped_off(p);
                        break Ok(macros_utils::Match::Many(matches));
                    };
                    if r.is_ok() {
                        break r;
                    }
                }
            });
            quote! {
                loop {
                    #(#choices)*
                    break Err(macros_utils::MacrosError::Parse(macros_utils::ParseError::new(stream.peek().map(|t| t.span()).unwrap_or_else(macros_utils::call_site), macros_utils::ParseErrorKind::NoMatchingChoice)));
                }
            }
        },
        Pattern::Group(delimiter, patterns) => {
            let delimiter = match delimiter {
                macros_utils::Delimiter::Parenthesis => quote! { Parenthesis },
                macros_utils::Delimiter::Brace => quote! { Brace },
                macros_utils::Delimiter::Bracket => quote! { Bracket },
                macros_utils::Delimiter::None => quote! { None },
            };
            let patterns = patterns
                .into_iter()
                .map(|p| pattern_statement(p, params))
                .map(|statement| {
                    quote! {
                        let r: Result<macros_utils::Match, macros_utils::MacrosError> = #statement;
                        if let Err(r) = r {
                            break Err(r);
                        }
                        matches.push(r.unwrap());
                    }
                });
            quote! {
                loop {
                    let token = stream.pop_or_err();
                    if let Err(e) = token {
                        break Err(macros_utils::MacrosError::Parse(e));
                    }
                    let token = token.unwrap();
                    if let macros_utils::Token::Group { delimiter: macros_utils::Delimiter::#delimiter, mut stream, .. } = token {
                        break loop {
                            let mut matches = vec![];
                            let p = {
                                let mut stream = stream.fork();
                                #(#patterns)*
                                if !stream.is_empty() {
                                    break Err(macros_utils::MacrosError::Parse(macros_utils::ParseError::new(stream.peek().map(|t| t.span()).unwrap_or_else(macros_utils::call_site), macros_utils::ParseErrorKind::InputTooLong)));
                                }
                                stream.popped()
                            };
                            // stream variable is now the original stream, need to pop off the tokens
                            stream.popped_off(p);
                            break Ok(macros_utils::Match::Many(matches));
                            };
                    } else {
                        break Err(macros_utils::MacrosError::Parse(macros_utils::ParseError::new(token.span(), macros_utils::ParseErrorKind::ExpectedGroup(macros_utils::Delimiter::#delimiter))));
                    }
                }
            }
        },
        Pattern::OneOrMore(patterns) => {
            let patterns = patterns
                .into_iter()
                .map(|p| pattern_statement(p, params))
                .map(|statement| {
                    quote! {
                        let r: Result<macros_utils::Match, macros_utils::MacrosError> = #statement;
                        if let Err(r) = r {
                            break Err(r);
                        }
                        m.push(r.unwrap());
                    }
                });
            quote! {
                {
                    let mut matches = vec![];
                    let _ = loop {
                        let mut m = vec![];
                        let p = {
                            let mut stream = stream.fork();
                            if stream.is_empty() {
                                break Ok(());
                            }
                            #(#patterns)*
                            stream.popped()
                        };
                        // stream variable is now the original stream, need to pop off the tokens
                        stream.popped_off(p);
                        matches.push(macros_utils::Match::Many(m));
                    };
                    if matches.is_empty() {
                        Err(macros_utils::MacrosError::Parse(macros_utils::ParseError::new(stream.peek().map(|t| t.span()).unwrap_or_else(macros_utils::call_site), macros_utils::ParseErrorKind::ExpectedRepetition)))
                    } else {
                        Ok(macros_utils::Match::Many(matches))
                    }
                }
            }
        },
        Pattern::ZeroOrMore(patterns) => {
            let patterns = patterns
                .into_iter()
                .map(|p| pattern_statement(p, params))
                .map(|statement| {
                    quote! {
                        let r: Result<macros_utils::Match, macros_utils::MacrosError> = #statement;
                        if let Err(r) = r {
                            break Err(r);
                        }
                        m.push(r.unwrap());
                    }
                });
            quote! {
                {
                    let mut matches = vec![];
                    let _ = loop {
                        let mut m = vec![];
                        let p = {
                            let mut stream = stream.fork();
                            if stream.is_empty() {
                                break Ok(());
                            }
                            #(#patterns)*
                            stream.popped()
                        };
                        // stream variable is now the original stream, need to pop off the tokens
                        stream.popped_off(p);
                        matches.push(macros_utils::Match::Many(m));
                    };
                    if matches.is_empty() {
                        Ok(macros_utils::Match::None)
                    } else {
                        Ok(macros_utils::Match::Many(matches))
                    }
                }
            }
        },
        Pattern::Optional(patterns) => {
            let patterns = patterns
                .into_iter()
                .map(|p| pattern_statement(p, params))
                .map(|statement| {
                    quote! {
                        let r: Result<macros_utils::Match, macros_utils::MacrosError> = #statement;
                        if let Err(r) = r {
                            break Err(r);
                        }
                        matches.push(r.unwrap());
                    }
                });
            quote! {
                {
                    let r: Result<macros_utils::Match, macros_utils::MacrosError> = loop {
                        let mut matches = vec![];
                        let p = {
                            let mut stream = stream.fork();
                            #(#patterns)*
                            stream.popped()
                        };
                        // stream variable is now the original stream, need to pop off the tokens
                        stream.popped_off(p);
                        break Ok(macros_utils::Match::Many(matches));
                    };
                    if r.is_err() {
                        Ok(macros_utils::Match::None)
                    } else {
                        Ok(r.unwrap())
                    }
                }
            }
        },
        Pattern::Parameter(patterns, name) => {
            let patterns = patterns
                .into_iter()
                .map(|p| pattern_statement(p, params))
                .map(|statement| {
                    quote! {
                        let r: Result<macros_utils::Match, macros_utils::MacrosError> = #statement;
                        if let Err(r) = r {
                            break Err(r);
                        }
                        matches.push(r.unwrap());
                    }
                });
            let name = Token::Ident {
                name,
                span: macros_utils::call_site(),
            };
            let optional = params.iter().any(|p| p.0 == name && p.1);
            let assign = if optional {
                quote! { Some(r.as_ref().unwrap().clone()) }
            } else {
                quote! { r.as_ref().unwrap().clone() }
            };
            quote! {
                {
                    let r: Result<macros_utils::Match, macros_utils::MacrosError> = loop {
                        let mut matches = vec![];
                        let p = {
                            let mut stream = stream.fork();
                            #(#patterns)*
                            stream.popped()
                        };
                        // stream variable is now the original stream, need to pop off the tokens
                        stream.popped_off(p);
                        break Ok(macros_utils::Match::Many(matches));
                    };
                    if r.is_ok() {
                        if let Ok(macros_utils::Match::None) = r {
                        } else {
                            #name = #assign;
                        }
                    }
                    r
                }
            }
        },
        Pattern::Token(token) => {
            let token = token.to_token_stream();
            quote! {
                loop {
                    let token = stream.pop_or_err();
                    if let Err(e) = token {
                        break Err(macros_utils::MacrosError::Parse(e));
                    }
                    let token = token.unwrap();
                    let t = #token;
                    break if token == t {
                        Ok(macros_utils::Match::One(token))
                    } else {
                        Err(macros_utils::MacrosError::Parse(macros_utils::ParseError::new(token.span(), macros_utils::ParseErrorKind::Expected(t, token))))
                    };
                }
            }
        },
        _ => quote! { Ok(macros_utils::Match::None) },
    }
}
