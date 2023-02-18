use crate::{ParseError, ParseErrorKind, ParseResult};

pub fn get_byte_at<B: AsRef<[u8]>>(b: B, index: usize) -> u8 {
    let b = b.as_ref();
    if index < b.len() {
        b[index]
    } else {
        0
    }
}

pub fn next_char(s: &str) -> char {
    s.chars().next().unwrap()
}

pub fn parse_lit_str(mut s: &str) -> ParseResult<(String, String)> {
    s = &s[1..];
    let mut string = String::new();
    'main: loop {
        let c = match get_byte_at(s, 0) {
            b'"' => break,
            b'\\' => {
                let b = get_byte_at(s, 1);
                s = &s[2..];
                match b {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'\\' => '\\',
                    b'0' => '\0',
                    b'\'' => '\'',
                    b'"' => '"',
                    b'x' => {
                        let c = parse_two_char_hex(s)?;
                        if c >= 0x80 {
                            return Err(ParseError::call_site(ParseErrorKind::InvalidByte(c)));
                        }
                        s = &s[2..];
                        char::from_u32(u32::from(c)).unwrap()
                    },
                    b'u' => {
                        let (c, r) = parse_unicode_in_braces(s)?;
                        s = r;
                        c
                    },
                    b'\r' | b'\n' => loop {
                        let c = next_char(s);
                        if c.is_whitespace() {
                            s = &s[c.len_utf8()..];
                        } else {
                            continue 'main;
                        }
                    },
                    b => {
                        return Err(ParseError::call_site(
                            ParseErrorKind::InvalidEscapeCharacter(b),
                        ))
                    },
                }
            },
            _ => {
                let c = next_char(s);
                s = &s[c.len_utf8()..];
                c
            },
        };
        string.push(c);
    }
    let suffix = s[1..].to_string();
    Ok((string, suffix))
}

pub fn parse_lit_str_raw(mut s: &str) -> ParseResult<(String, String, u8)> {
    s = &s[1..];
    let mut hashtags = 0;
    while get_byte_at(s, hashtags) == b'#' {
        hashtags += 1;
    }
    let end_quote = s.rfind('"').unwrap();
    let content = s[hashtags + 1..end_quote].to_string();
    let suffix = s[end_quote + 1..].to_string();
    Ok((content, suffix, hashtags as u8))
}

pub fn parse_lit_byte(s: &str) -> ParseResult<(String, String)> {
    parse_lit_char(s)
}

pub fn parse_lit_byte_str(s: &str) -> ParseResult<(String, String)> {
    parse_lit_str(s)
}

pub fn parse_lit_byte_str_raw(s: &str) -> ParseResult<(String, String, u8)> {
    parse_lit_str_raw(s)
}

pub fn parse_lit_char(mut s: &str) -> ParseResult<(String, String)> {
    s = &s[1..];
    let c = match get_byte_at(s, 0) {
        b'\\' => {
            let b = get_byte_at(s, 1);
            s = &s[2..];
            match b {
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b'\\' => '\\',
                b'0' => '\0',
                b'\'' => '\'',
                b'"' => '"',
                b'x' => {
                    let c = parse_two_char_hex(s)?;
                    if c >= 0x80 {
                        return Err(ParseError::call_site(ParseErrorKind::InvalidByte(c)));
                    }
                    s = &s[2..];
                    char::from_u32(u32::from(c)).unwrap()
                },
                b'u' => {
                    let (c, r) = parse_unicode_in_braces(s)?;
                    s = r;
                    c
                },
                b => {
                    return Err(ParseError::call_site(
                        ParseErrorKind::InvalidEscapeCharacter(b),
                    ))
                },
            }
        },
        _ => {
            let c = next_char(s);
            s = &s[c.len_utf8()..];
            c
        },
    };
    let suffix = s[1..].to_string();
    Ok((c.into(), suffix))
}

pub fn parse_lit_int(mut s: &str) -> ParseResult<(String, String)> {
    let is_negative = get_byte_at(s, 0) == b'-';
    if is_negative {
        s = &s[1..];
    }
    let base = match ((get_byte_at(s, 0), get_byte_at(s, 1)), get_byte_at(s, 2)) {
        ((b'0', b'x'), _) => 16,
        ((b'0', b'o'), _) => 8,
        ((b'0', b'b'), _) => 2,
        _ => 10,
    };
    if base != 10 {
        s = &s[2..];
    }
    let mut value: u128 = 0;
    loop {
        let byte = get_byte_at(s, 0);
        let v = match byte {
            b'0'..=b'1' if base == 2 => byte - b'0',
            b'0'..=b'7' if base == 8 => byte - b'0',
            b'0'..=b'9' if base >= 10 => byte - b'0',
            b'a'..=b'f' if base > 10 => byte - b'a' + 10,
            b'A'..=b'F' if base > 10 => byte - b'A' + 10,
            b'_' => {
                s = &s[1..];
                continue;
            },
            b'e' | b'E' => {
                return Err(ParseError::call_site(ParseErrorKind::SuffixNoE));
            },
            _ => break,
        };
        if v >= base {
            return Err(ParseError::call_site(ParseErrorKind::InvalidDigit(v, base)));
        }
        value *= base as u128;
        value += v as u128;
        s = &s[1..];
    }
    let suffix = s.to_string();
    Ok((
        format!("{}{}", if is_negative { "-" } else { "" }, value),
        suffix,
    ))
}

/// Use this first to check if it's a float (has a `.`)
/// If it returns `None`, it's not a float and must be an int
pub fn parse_lit_float(mut s: &str) -> ParseResult<Option<(String, String)>> {
    if !s.contains('.') {
        return Ok(None);
    }
    let mut string = String::new();
    let mut decimal_seen = false;
    let mut exponent_seen = false;
    let mut exponent_sign_seen = false;
    let mut exponent_digits_seen = false;
    loop {
        match get_byte_at(s, 0) {
            b'_' => {
                s = &s[1..];
            },
            b'0'..=b'9' => {
                if exponent_seen {
                    exponent_digits_seen = true;
                }
                string.push(next_char(s));
                s = &s[1..];
            },
            b'.' => {
                if decimal_seen {
                    return Err(ParseError::call_site(
                        ParseErrorKind::MultipleDecimalPointsInFloat,
                    ));
                }
                decimal_seen = true;
                string.push(next_char(s));
                s = &s[1..];
            },
            b'e' | b'E' => {
                if exponent_seen {
                    return Err(ParseError::call_site(
                        ParseErrorKind::MultipleExponentsInFloat,
                    ));
                }
                exponent_seen = true;
                string.push(next_char(s));
                s = &s[1..];
            },
            b'+' | b'-' => {
                if !exponent_seen {
                    return Err(ParseError::call_site(ParseErrorKind::UnexpectedSignInFloat));
                }
                if exponent_sign_seen {
                    return Err(ParseError::call_site(ParseErrorKind::MultipleSignsInFloat));
                }
                exponent_sign_seen = true;
                string.push(next_char(s));
                s = &s[1..];
            },
            _ => {
                if exponent_seen && !exponent_digits_seen {
                    return Err(ParseError::call_site(ParseErrorKind::MissingExponentDigits));
                }
                break;
            },
        }
    }
    let suffix = s.to_string();
    Ok(Some((string, suffix)))
}

pub fn parse_two_char_hex(s: &str) -> ParseResult<u8> {
    // first byte is 10 times value
    // second byte is 1 times value
    let first = get_byte_at(s, 0);
    let second = get_byte_at(s, 1);
    Ok(0x10
        * match first {
            b'0'..=b'9' => first - b'0',
            b'a'..=b'f' => first - b'a' + 10,
            b'A'..=b'F' => first - b'A' + 10,
            b => return Err(ParseError::call_site(ParseErrorKind::InvalidByte(b))),
        }
        + match second {
            b'0'..=b'9' => second - b'0',
            b'a'..=b'f' => second - b'a' + 10,
            b'A'..=b'F' => second - b'A' + 10,
            b => return Err(ParseError::call_site(ParseErrorKind::InvalidByte(b))),
        })
}

pub fn parse_unicode_in_braces(mut s: &str) -> ParseResult<(char, &str)> {
    if get_byte_at(s, 0) != b'{' {
        return Err(ParseError::call_site(
            ParseErrorKind::MissingUnicodeOpeningBrace,
        ));
    }
    s = &s[1..];
    let mut c = 0;
    let mut digits = 0;
    loop {
        let byte = get_byte_at(s, digits);
        if byte == b'}' {
            break;
        }
        if digits == 6 {
            return Err(ParseError::call_site(ParseErrorKind::TooManyUnicodeDigits));
        }
        let v = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            b => return Err(ParseError::call_site(ParseErrorKind::InvalidByte(b))),
        };
        c = c * 16 + (v as u32);
        digits += 1;
    }
    if digits == 0 {
        return Err(ParseError::call_site(ParseErrorKind::MissingUnicodeDigits));
    }
    Ok((char::from_u32(c).unwrap(), &s[digits + 1..]))
}
