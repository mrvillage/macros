pub use macros_macros::parser;
pub struct TestParser {
    // help: Vec<macros_utils::MacroStream>,
}
#[automatically_derived]
impl ::core::clone::Clone for TestParser {
    #[inline]
    fn clone(&self) -> TestParser {
        TestParser {
            // help: ::core::clone::Clone::clone(&self.help),
        }
    }
}
#[allow(clippy::never_loop)]
impl macros_utils::Parse for TestParser {
    fn parse(stream: &mut macros_utils::MacroStream) -> Result<Self, macros_utils::MacrosError> {
        loop {
            let token = stream.pop_or_err();
            if let Err(e) = token {
                break Err(macros_utils::MacrosError::Parse(e));
            }
            let token = token.unwrap();
            if let macros_utils::Token::Group {
                delimiter: macros_utils::Delimiter::Brace,
                mut stream,
                ..
            } = token
            {
                break loop {
                    let p = {
                        let mut stream = stream.fork();
                        let r: Result<Vec<macros_utils::MacroStream>, macros_utils::MacrosError> = {
                            loop {
                                let token = stream.pop_or_err();
                                if let Err(e) = token {
                                    break Err(macros_utils::MacrosError::Parse(e));
                                }
                                let token = token.unwrap();
                                let t = macros_utils::Token::Ident {
                                    name: "hi".to_string(),
                                    span: macros_utils::call_site(),
                                };
                                break if token == t {
                                    let mut s = macros_utils::MacroStream::new();
                                    s.push_front(token);
                                    Ok(<[_]>::into_vec(Box::new([s])))
                                } else {
                                    Err(macros_utils::MacrosError::Parse(
                                        macros_utils::ParseError::new(
                                            token.span(),
                                            macros_utils::ParseErrorKind::Expected(t, token),
                                        ),
                                    ))
                                };
                            }
                        };
                        if let Err(r) = r {
                            break Err(r);
                        }
                        if !stream.is_empty() {
                            break Err(macros_utils::MacrosError::Parse(
                                macros_utils::ParseError::new(
                                    stream
                                        .peek()
                                        .map(|t| t.span())
                                        .unwrap_or_else(macros_utils::call_site),
                                    macros_utils::ParseErrorKind::InputTooLong,
                                ),
                            ));
                        }
                        stream.popped()
                    };
                    let s = stream.popped_off_fork(p);
                    break Ok(<[_]>::into_vec(Box::new([s])));
                };
            } else {
                break Err(macros_utils::MacrosError::Parse(
                    macros_utils::ParseError::new(
                        token.span(),
                        macros_utils::ParseErrorKind::ExpectedGroup(macros_utils::Delimiter::Brace),
                    ),
                ));
            }
        }?;
        Ok(Self {})
    }
}
