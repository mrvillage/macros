mod input;

use input::ParserInput;
use proc_macro2::TokenStream;
use quote::quote;

#[proc_macro]
pub fn parser(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input
}
