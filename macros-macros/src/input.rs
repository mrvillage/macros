pub struct ParserInput {
    pub input: String,
    pub patterns: Vec<Pattern>,
}

/// {...}? indicates optional
/// {name} indicates a parameter
/// {...}* indicates zero or more
/// {...}+ indicates one or more
/// ...:name indicates a boolean to set true if the pattern is matched
/// {...|...|...} indicates a choice
pub enum Pattern {
    Optional(Box<Pattern>),
    Parameter(String),
    ZeroOrMore(Box<Pattern>),
    OneOrMore(Box<Pattern>),
    Boolean(Box<Pattern>, String),
    Choice(Vec<Pattern>),
    // Token(String),
}
