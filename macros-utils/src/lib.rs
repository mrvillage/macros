mod error;
mod parsers;

use std::collections::VecDeque;

pub use error::{MacrosError, MacrosErrorKind};
use proc_macro2::{TokenStream, TokenTree};

#[derive(Debug)]
struct MacroStream {
    stream: VecDeque<TokenTree>,
}

impl MacroStream {
    fn new(stream: TokenStream) -> Self {
        let mut vec = VecDeque::new();
        for i in stream.into_iter() {
            vec.push_back(i);
        }
        Self { stream: vec }
    }

    fn next(&mut self) -> Option<TokenTree> {
        let t = self.stream.pop_front();
        t
    }

    fn peek(&self) -> Option<&TokenTree> {
        self.stream.front()
    }
}
