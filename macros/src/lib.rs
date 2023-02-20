pub use macros_macros::parser;

parser! {TestParser =>
    {{}$ : help}@
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use macros_utils::Parse;
    use proc_macro2::TokenStream;

    #[test]
    fn it_works() {
        let r = TestParser::parse(&mut TokenStream::from_str("chicken").unwrap().into()).unwrap();
        panic!("PARSED {:?}", r);
    }
}
