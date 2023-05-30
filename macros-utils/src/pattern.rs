use std::{borrow::Cow, str::FromStr};

use crate::{
    call_site, Delimiter, MacroStream, MacrosError, Match, Parse, ParseError, ParseErrorKind,
    ParserOutput, Spacing, Token,
};
use proc_macro2::TokenStream;
use proc_macro_error::{abort, abort_call_site};

#[doc(hidden)]
pub struct ParserInput<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    pub patterns: Vec<Pattern<T>>,
}

/// A pattern to match against a `MacroStream` in a parser from the `parser!` macro.
///
/// The following are the various patterns that can be used:
/// - {...}? indicates that the pattern is optional
/// - {... : name : type}@ indicates that the match should be bound to the parameter `name` with the type `type`, the type can be any type that
/// - {...}* indicates zero or more (non-greedy), meaning it will consume the stream until the next pattern matches
/// - {...}** indicates zero or more (greedy), meaning it will consume the remainder of the stream
/// - {...}+ indicates one or more (non-greedy), meaning it will consume the stream until the next pattern matches
/// - {...}++ indicates one or more (greedy), meaning it will consume the remainder of the stream
/// - {... | ... | ...}& indicates a choice
/// - ... indicates a token to match exactly
/// - {}$ indicates an arbitrary token, if used in a zero or more or one or more then it will consume the stream until the next pattern matches
/// - {...}= indicates a validation function, should be anything of type type `for<'a> fn(Cow<'a, T>, &Match) -> (Result<(), String>, Cow<'a, T>)` as it will be interpolated directly into the code expecting that type. Validation functions will receive the current output and the previous match, and should return the new output (allowing modification) and an optional error.
/// - {{...}} escapes the {} grouping
/// - To escape any of the special endings, use ~whatever before the ending, to escape the tilde use ~~
pub enum Pattern<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    Optional(Vec<Pattern<T>>),
    Parameter(Vec<Pattern<T>>, String, MacroStream),
    ZeroOrMore(Vec<Pattern<T>>, bool),
    OneOrMore(Vec<Pattern<T>>, bool),
    Choice(Vec<Vec<Pattern<T>>>),
    Token(Token),
    Group(Delimiter, Vec<Pattern<T>>),
    Any,
    #[allow(clippy::type_complexity)]
    Validator(
        Option<MacroStream>,
        Option<for<'a> fn(Cow<'a, T>, &Match) -> (Result<(), String>, Cow<'a, T>)>,
    ),
}

impl<T> ParserInput<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    pub fn params(&self) -> Vec<(String, bool, bool, MacroStream)> {
        let mut params = vec![];
        for pattern in &self.patterns {
            params.extend(pattern.params());
        }
        params
    }
}

impl<T> Parse for ParserInput<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    fn parse(stream: &mut MacroStream) -> Result<Self, MacrosError> {
        Ok(Self {
            patterns: stream_to_patterns(stream)?,
        })
    }
}

fn stream_to_patterns<T>(stream: &mut MacroStream) -> Result<Vec<Pattern<T>>, MacrosError>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    let mut patterns = Vec::new();
    let mut prev = None;
    while !stream.is_empty() {
        let current = Pattern::parse(stream)?;
        if let (Some(&Pattern::Validator(_, _)) | None, Pattern::Validator(_, _)) = (prev, &current)
        {
            return Err(ParseError::new(
                stream.peek().map(|t| t.span()).unwrap_or_else(call_site),
                ParseErrorKind::InvalidValidatorPosition,
            )
            .into());
        }
        patterns.push(current);
        prev = Some(patterns.last().unwrap());
    }
    Ok(patterns)
}

impl<T> Parse for Pattern<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
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
                                Self::ZeroOrMore(stream_to_patterns(&mut stream)?, false)
                            },
                            Some(Token::Punctuation { value: '*', spacing: Spacing::Joint, .. }) => {
                                stream.push_front(token);
                                Self::ZeroOrMore(stream_to_patterns(&mut stream)?, match input.peek_at(1) {
                                    Some(Token::Punctuation { value: '*', spacing: Spacing::Alone, .. }) => {
                                        input.pop(); // pops the previous token off so that this one is popped off at the end of the match
                                        true
                                    },
                                    _ => false
                                })

                            }
                            Some(Token::Punctuation { value: '+', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                Self::OneOrMore(stream_to_patterns(&mut stream)?, false)
                            },
                            Some(Token::Punctuation { value: '+', spacing: Spacing::Joint, .. }) => {
                                stream.push_front(token);
                                Self::OneOrMore(stream_to_patterns(&mut stream)?, match input.peek_at(1) {
                                    Some(Token::Punctuation { value: '+', spacing: Spacing::Alone, .. }) => {
                                        input.pop(); // pops the previous token off so that this one is popped off at the end of the match
                                        true
                                    },
                                    _ => false
                                })
                            }
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
                                    Token::Ident { name, .. } => {
                                        let type_ = match stream.pop() {
                                            Some(Token::Punctuation { value: ':', spacing: Spacing::Alone, span }) => {
                                                if stream.is_empty() {
                                                    abort!(span, "expected a type after the colon, found end of input");
                                                }
                                                stream
                                            },
                                            Some(_) => abort!(span, "expected a colon after the identifier"),
                                            None => MacroStream::from_tokens(TokenStream::from_str("macros_core::Match").unwrap()).unwrap(),
                                        };
                                        Self::Parameter(patterns, name, type_)
                                    },
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
                                Self::Any
                            },
                            Some(Token::Punctuation { value: '=', spacing: Spacing::Alone, .. }) => {
                                stream.push_front(token);
                                Self::Validator(Some(stream), None)
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
                            Self::Any
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

impl<T> Pattern<T>
where
    T: ToOwned<Owned = T> + ParserOutput,
{
    pub fn params(&self) -> Vec<(String, bool, bool, MacroStream)> {
        let mut params = vec![];
        match self {
            Self::Group(_, patterns) => {
                for i in patterns {
                    params.extend(i.params());
                }
            },
            Self::Optional(patterns) => {
                for i in patterns {
                    params.extend(
                        i.params()
                            .into_iter()
                            .map(|(name, _, variadic, type_)| (name, true, variadic, type_)),
                    );
                }
            },
            Self::ZeroOrMore(patterns, _) => {
                for i in patterns {
                    params.extend(
                        i.params()
                            .into_iter()
                            .map(|(name, optional, _, type_)| (name, optional, true, type_)),
                    );
                }
            },
            Self::OneOrMore(patterns, _) => {
                for i in patterns {
                    params.extend(
                        i.params()
                            .into_iter()
                            .map(|(name, optional, _, type_)| (name, optional, true, type_)),
                    );
                }
            },
            Self::Choice(patterns) => {
                for i in patterns {
                    for j in i {
                        params.extend(j.params());
                    }
                }
            },
            Self::Parameter(patterns, name, type_) => {
                for i in patterns {
                    params.extend(i.params());
                }
                params.push((name.clone(), false, false, type_.clone()));
            },
            _ => {},
        };
        params
    }

    pub fn match_pattern<'a>(
        &self,
        mut output: Cow<'a, T>,
        next: Option<&Pattern<T>>,
        next2: Option<&Pattern<T>>,
        stream: &mut MacroStream,
    ) -> (Result<Match, MacrosError>, Cow<'a, T>) {
        let match_next = match next {
            Some(Pattern::Validator(_, _)) => next2,
            _ => next,
        };
        let res = match self {
            Self::Any => (
                stream
                    .pop_or_err()
                    .map(Match::One)
                    .map_err(MacrosError::Parse),
                output,
            ),
            Self::Choice(choices) => {
                'choice: for choice in choices {
                    let mut fork = stream.fork();
                    let (res, o) = Self::match_patterns(output, choice, &mut fork);
                    if res.is_err() {
                        output = o;
                        continue 'choice;
                    }
                    stream.unfork(fork);
                    return (res, o);
                }
                (
                    Err(MacrosError::Parse(ParseError::new(
                        stream.peek().map(|t| t.span()).unwrap_or_else(call_site),
                        ParseErrorKind::NoMatchingChoice,
                    ))),
                    output,
                )
            },
            Self::Group(delimiter, patterns) => {
                let token = match stream.pop_or_err().map_err(MacrosError::Parse) {
                    Ok(token) => token,
                    Err(e) => return (Err(e), output),
                };
                if let Token::Group {
                    delimiter: d,
                    stream: s,
                    ..
                } = &token
                {
                    if d == delimiter {
                        let mut fork = s.fork();
                        let (res, o) = Self::match_patterns(output, patterns, &mut fork);
                        if !fork.is_empty() {
                            return (
                                Err(MacrosError::Parse(ParseError::new(
                                    stream.peek().map(|t| t.span()).unwrap_or_else(call_site),
                                    ParseErrorKind::InputTooLong,
                                ))),
                                o,
                            );
                        }
                        stream.unfork(fork);
                        return (res, o);
                    }
                }
                (
                    Err(MacrosError::Parse(ParseError::new(
                        token.span(),
                        ParseErrorKind::ExpectedGroup(*delimiter),
                    ))),
                    output,
                )
            },
            Self::OneOrMore(patterns, greedy) => {
                let mut matches = vec![];
                loop {
                    let mut fork = stream.fork();
                    match Self::match_patterns(output, patterns, &mut fork) {
                        (Ok(m), o) => {
                            stream.unfork(fork);
                            matches.push(m);
                            output = o;
                        },
                        (Err(e), o) => {
                            output = o;
                            if matches.is_empty() {
                                return (Err(e), output);
                            }
                            break;
                        },
                    }
                    let mut fork = stream.fork();
                    match match_next {
                        Some(next) if !greedy && !matches.is_empty() => {
                            match next.match_pattern(output, None, None, &mut fork) {
                                (Ok(_), o) => {
                                    output = o;
                                    break;
                                },
                                (_, o) => output = o,
                            }
                        },
                        _ => {},
                    }
                }
                (
                    if matches.is_empty() {
                        Err(MacrosError::Parse(ParseError::new(
                            stream.peek().map(|t| t.span()).unwrap_or_else(call_site),
                            ParseErrorKind::ExpectedRepetition,
                        )))
                    } else {
                        Ok(Match::Many(matches))
                    },
                    output,
                )
            },
            Self::ZeroOrMore(patterns, greedy) => {
                let mut matches = vec![];
                loop {
                    let mut fork = stream.fork();
                    match Self::match_patterns(output, patterns, &mut fork) {
                        (Ok(m), o) => {
                            stream.unfork(fork);
                            matches.push(m);
                            output = o;
                        },
                        (_, o) => {
                            output = o;
                            break;
                        },
                    }
                    let mut fork = stream.fork();
                    match match_next {
                        Some(next) if !greedy => {
                            match next.match_pattern(output.clone(), None, None, &mut fork) {
                                (Ok(_), o) => {
                                    output = o;
                                    break;
                                },
                                (_, o) => output = o,
                            }
                        },
                        _ => {},
                    }
                }
                (
                    if matches.is_empty() {
                        Ok(Match::None)
                    } else {
                        Ok(Match::Many(matches))
                    },
                    output,
                )
            },
            Self::Optional(patterns) => {
                let mut fork = stream.fork();
                match Self::match_patterns(output.clone(), patterns, &mut fork) {
                    r @ (Ok(_), _) => {
                        stream.unfork(fork);
                        r
                    },
                    (_, o) => (Ok(Match::None), o),
                }
            },
            Self::Token(token) => (
                match stream.pop_or_err().map_err(MacrosError::Parse) {
                    Ok(t) if t == *token => Ok(Match::One(t)),
                    Ok(t) => Err(MacrosError::Parse(ParseError::new(
                        t.span(),
                        ParseErrorKind::Expected(token.clone(), t),
                    ))),
                    Err(e) => Err(e),
                },
                output,
            ),
            Self::Parameter(patterns, name, _) => {
                let mut fork = stream.fork();
                let (res, mut o) = Self::match_patterns(output, patterns, &mut fork);
                match res {
                    Ok(m) => {
                        stream.unfork(fork);
                        if let Err(e) = o.to_mut().set_match(name, m.clone()) {
                            (Err(e), o)
                        } else {
                            (Ok(m), o)
                        }
                    },
                    Err(e) => (Err(e), o),
                }
            },
            Self::Validator(_, _) => panic!(
                "Validator pattern should not have been passed into `Pattern::match_pattern`"
            ),
        };
        match (next, res) {
            (Some(Pattern::Validator(_, Some(f))), (Ok(m), output)) => match f(output, &m) {
                (Ok(_), o) => (Ok(m), o),
                (Err(e), o) => (
                    Err(MacrosError::Parse(ParseError::new(
                        stream.peek().map(|t| t.span()).unwrap_or_else(call_site),
                        ParseErrorKind::ValidatorFailed(e),
                    ))),
                    o,
                ),
            },
            (_, m) => m,
        }
    }

    pub fn match_patterns<'b, 'a: 'b>(
        mut output: Cow<'a, T>,
        patterns: &'b [Pattern<T>],
        stream: &mut MacroStream,
    ) -> (Result<Match, MacrosError>, Cow<'a, T>) {
        let mut matches = vec![];
        for (i, pattern) in patterns.iter().enumerate() {
            if let Pattern::Validator(_, _) = pattern {
                continue;
            }
            match pattern.match_pattern(output, patterns.get(i + 1), patterns.get(i + 2), stream) {
                (Ok(m @ Match::One(_)), o) => {
                    matches.push(m);
                    output = o;
                },
                (Ok(Match::None), o) => output = o,
                (Ok(Match::Many(m)), o) => {
                    matches.extend(m);
                    output = o;
                },
                e => return e,
            }
        }
        (Ok(Match::Many(matches)), output)
    }
}

unsafe impl<T> Sync for Pattern<T> where T: ToOwned<Owned = T> + ParserOutput {}
