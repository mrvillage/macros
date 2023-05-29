pub use macros_macros::parser;
pub use macros_utils::*;

parser! {
    Test => { hello : param }@
}

parser! {
    TestParser => {}$ { {}$ : param }@
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parser() {
        let output: TestParser = TestParser::parse(
            &mut proc_macro2::TokenStream::from_str("hi hello")
                .unwrap()
                .into(),
        )
        .unwrap();
        println!("{:?}", output.param)
    }
}
