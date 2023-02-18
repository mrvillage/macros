use crate::{MacroStream, MacrosError};

pub trait Parse: Sized {
    fn parse(input: &mut MacroStream) -> Result<Self, MacrosError>;
}
