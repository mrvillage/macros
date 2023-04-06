mod error;
mod parse;
mod parsers;
mod tokens;

use std::collections::VecDeque;

pub use error::{MacrosError, ParseError, ParseErrorKind, ToMacrosError};
pub use parse::Parse;
use proc_macro2::TokenStream;
pub use proc_macro2::{Spacing, Span};
use quote::ToTokens;
pub use tokens::{Delimiter, Token};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroStream {
    stream: VecDeque<Token>,
    popped: usize,
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

#[derive(Clone, Debug)]
pub enum Match {
    One(Token),
    Many(Vec<Match>),
    None,
}

impl MacroStream {
    pub fn new() -> Self {
        Self {
            stream: VecDeque::new(),
            popped: 0,
        }
    }

    pub fn popped(&self) -> usize {
        self.popped
    }

    pub fn from_tokens(stream: TokenStream) -> ParseResult<Self> {
        let mut tokens = VecDeque::new();
        for i in stream.into_iter() {
            tokens.push_back(i);
        }
        let mut stream = VecDeque::new();
        while !tokens.is_empty() {
            stream.push_back(Token::from_tokens(&mut tokens)?);
        }
        Ok(Self { stream, popped: 0 })
    }

    pub fn pop(&mut self) -> Option<Token> {
        self.stream.pop_front()
    }

    pub fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    pub fn peek_at(&self, i: usize) -> Option<&Token> {
        self.stream.get(i)
    }

    pub fn parse<T>(&mut self) -> Result<T, MacrosError>
    where
        T: Parse,
    {
        T::parse(self)
    }

    pub fn is_empty(&self) -> bool {
        self.stream.is_empty()
    }

    pub fn pop_or_err(&mut self) -> Result<Token, ParseError> {
        self.pop()
            .ok_or_else(|| {
                ParseError::call_site(ParseErrorKind::UnexpectedEndOfInput("".to_string()))
            })
            .map(|i| {
                self.popped += 1;
                i
            })
    }

    pub fn peek_or_err(&self) -> Result<&Token, ParseError> {
        self.peek().ok_or_else(|| {
            ParseError::call_site(ParseErrorKind::UnexpectedEndOfInput("".to_string()))
        })
    }

    pub fn push_front(&mut self, token: Token) {
        self.stream.push_front(token)
    }

    pub fn push_back(&mut self, token: Token) {
        self.stream.push_back(token)
    }

    pub fn len(&self) -> usize {
        self.stream.len()
    }

    pub fn fork(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            popped: 0,
        }
    }

    pub fn popped_off_fork(&mut self, p: usize) -> Self {
        let mut popped = Self::new();
        for _ in 0..p {
            self.popped += 1;
            popped.push_back(self.pop().unwrap());
        }
        popped
    }

    pub fn popped_off(&mut self, p: usize) {
        for _ in 0..p {
            self.pop().unwrap();
        }
    }
}

impl From<TokenStream> for MacroStream {
    fn from(stream: TokenStream) -> Self {
        Self::from_tokens(stream).unwrap()
    }
}

impl Default for MacroStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ToTokens for MacroStream {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for i in &self.stream {
            i.to_tokens(tokens);
        }
    }
}

pub fn call_site() -> Span {
    Span::call_site()
}
