mod error;
mod parsers;
mod tokens;

use std::collections::VecDeque;

pub use error::{MacrosError, MacrosErrorKind};
use proc_macro2::TokenStream;
pub use tokens::Token;

#[derive(Debug)]
pub struct MacroStream {
    stream: VecDeque<Token>,
}

impl MacroStream {
    pub fn new(stream: TokenStream) -> Self {
        let mut tokens = VecDeque::new();
        for i in stream.into_iter() {
            tokens.push_back(i);
        }
        let mut stream = VecDeque::new();
        while !tokens.is_empty() {
            stream.push_back(Token::from_tokens(&mut tokens));
        }
        Self { stream }
    }

    pub fn pop(&mut self) -> Option<Token> {
        self.stream.pop_front()
    }

    pub fn peek(&self) -> Option<&Token> {
        self.stream.front()
    }
}

impl From<TokenStream> for MacroStream {
    fn from(stream: TokenStream) -> Self {
        Self::new(stream)
    }
}
