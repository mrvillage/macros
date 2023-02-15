mod error;
mod parsers;
mod tokens;

use std::collections::VecDeque;

pub use error::{ParseError, ParseErrorKind};
use proc_macro2::TokenStream;
pub use tokens::Token;

#[derive(Debug)]
pub struct MacroStream {
    stream: VecDeque<Token>,
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl MacroStream {
    pub fn from_tokens(stream: TokenStream) -> ParseResult<Self> {
        let mut tokens = VecDeque::new();
        for i in stream.into_iter() {
            tokens.push_back(i);
        }
        let mut stream = VecDeque::new();
        while !tokens.is_empty() {
            stream.push_back(Token::from_tokens(&mut tokens)?);
        }
        Ok(Self { stream })
    }

    pub fn pop(&mut self) -> Option<Token> {
        self.stream.pop_front()
    }

    pub fn peek(&self) -> Option<&Token> {
        self.stream.front()
    }
}
