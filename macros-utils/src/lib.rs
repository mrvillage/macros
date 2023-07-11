mod error;
mod parse;
mod parsers;
mod pattern;
mod repr;
mod tokens;

use std::collections::VecDeque;

pub use error::{MacrosError, ParseError, ParseErrorKind};
pub use lazy_static::lazy_static;
pub use parse::Parse;
pub use pattern::{ParserInput, Pattern};
use proc_macro2::TokenStream;
pub use proc_macro2::{Spacing, Span};
use quote::ToTokens;
pub use repr::Repr;
pub use tokens::{Delimiter, LiteralKind, Token};

/// A stream of tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroStream {
    stream: VecDeque<Token>,
    popped: usize,
}

/// Type alias for the result of parsing to a `MacroStream`.
pub type ParseResult<T> = std::result::Result<T, ParseError>;

/// A match of a `Pattern`.
#[derive(Clone, Debug)]
pub enum Match {
    One(Token),
    Many(Vec<Match>),
    None,
}

impl Default for Match {
    fn default() -> Self {
        Self::None
    }
}

impl From<Match> for MacroStream {
    fn from(m: Match) -> MacroStream {
        let mut stream = MacroStream::new();
        match m {
            Match::One(i) => {
                stream.push_back(i);
            },
            Match::Many(m) => {
                for i in m {
                    stream.append(i.into());
                }
            },
            Match::None => {},
        }
        stream
    }
}

impl From<Match> for Option<MacroStream> {
    fn from(value: Match) -> Self {
        match value {
            Match::None => None,
            m => Some(m.into()),
        }
    }
}

impl MacroStream {
    /// Create a new empty `MacroStream`.
    pub fn new() -> Self {
        Self {
            stream: VecDeque::new(),
            popped: 0,
        }
    }

    /// Determine how many tokens have been popped from the stream.
    pub fn popped(&self) -> usize {
        self.popped
    }

    /// Create a `MacroStream` from a `proc_macro2::TokenStream`.
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

    pub fn from_vec(tokens: Vec<Token>) -> Self {
        Self {
            stream: tokens.into(),
            popped: 0,
        }
    }

    /// Pop a token from the stream.
    pub fn pop(&mut self) -> Option<Token> {
        self.stream.pop_front().map(|i| {
            self.popped += 1;
            i
        })
    }

    /// Peek at the next token in the stream.
    pub fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    /// Peek at the token at the given index in the stream.
    pub fn peek_at(&self, i: usize) -> Option<&Token> {
        self.stream.get(i)
    }

    /// Parse the stream into a type.
    pub fn parse<T>(&mut self) -> Result<T, MacrosError>
    where
        T: Parse,
    {
        T::parse(self)
    }

    /// Determine if the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.stream.is_empty()
    }

    /// Pop a token from the stream, or return an error if the stream is empty.
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

    /// Peek at the next token in the stream, or return an error if the stream is empty.
    pub fn peek_or_err(&self) -> Result<&Token, ParseError> {
        self.peek().ok_or_else(|| {
            ParseError::call_site(ParseErrorKind::UnexpectedEndOfInput("".to_string()))
        })
    }

    /// Push a token to the front of the stream.
    pub fn push_front(&mut self, token: Token) {
        self.stream.push_front(token)
    }

    /// Push a token to the back of the stream.
    pub fn push_back(&mut self, token: Token) {
        self.stream.push_back(token)
    }

    /// Get the length of the stream.
    pub fn len(&self) -> usize {
        self.stream.len()
    }

    /// Fork the stream (clone the stream and reset the popped count).
    pub fn fork(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            popped: 0,
        }
    }

    pub fn unfork(&mut self, other: Self) {
        self.stream = other.stream;
        self.popped = 0;
    }

    /// Pop a number of tokens from the stream.
    pub fn pop_many(&mut self, p: usize) {
        for _ in 0..p {
            self.pop().unwrap();
        }
    }

    pub fn append(&mut self, mut other: Self) {
        self.stream.append(&mut other.stream)
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

impl ToString for MacroStream {
    fn to_string(&self) -> String {
        let mut s = String::new();
        for i in &self.stream {
            s.push_str(&i.to_string());
        }
        s
    }
}

impl<T: Parse> TryFrom<Match> for (T,) {
    type Error = MacrosError;

    fn try_from(m: Match) -> Result<Self, Self::Error> {
        T::parse(&mut m.into()).map(|i| (i,))
    }
}

impl TryFrom<Match> for (Match,) {
    type Error = MacrosError;

    fn try_from(m: Match) -> Result<Self, Self::Error> {
        Ok((m,))
    }
}

/// A shortcut for `proc_macro2::Span::call_site()`.
#[inline(always)]
pub fn call_site() -> Span {
    Span::call_site()
}

/// The trait for the output of a parser created by the `parser!` macro.
pub trait ParserOutput {
    fn set_match(&mut self, k: &str, m: Match) -> Result<(), MacrosError>;
    fn name() -> &'static str;
}
