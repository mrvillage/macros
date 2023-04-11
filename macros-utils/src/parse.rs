use crate::{MacroStream, MacrosError};

/// Parse a `MacroStream` into a `Self`.
///
/// # Example
/// ```rs
/// use macros_utils::{Parse, MacroStream};
///
/// #[derive(Debug, Clone)]
/// struct MyStruct {
///     pub a: Token,
///     pub b: Token,
/// }
///
/// impl Parse for MyStruct {
///     fn parse(input: &mut MacroStream) -> Result<Self, MacrosError> {
///         let a = input.pop_or_err()?;
///         let b = input.pop_or_err()?;
///         Ok(Self { a, b })
///     }
/// }
pub trait Parse: Sized {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError>;
}
