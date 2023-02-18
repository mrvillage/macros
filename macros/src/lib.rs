pub use macros_macros::parser;
mod thing;

// parameter system is flawed, it doesn't retain any information on what exactly matched each pattern, just the overall tokenstream
// i think should probably redo how it's done slightly to allow nesting of patterns and things, like a Vec of Matches of something, each Match is a vector of streams or a single stream or something, or another vector of matches or something
parser! {TestParser =>
    {{hi}* : help}@
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use macros_utils::Parse;
    use proc_macro2::TokenStream;

    #[test]
    fn it_works() {
        let r = TestParser::parse(&mut TokenStream::from_str("~").unwrap().into()).unwrap();
        panic!("PARSED {:?}", r);
    }
}
