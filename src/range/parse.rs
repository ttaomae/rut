use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

use super::{CutRange, IncreasingRange, Ranges};

#[derive(Debug)]
pub(crate) enum Token {
    Number(usize),
    Hyphen,
    Comma,
    Blank(char),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{}", n),
            Token::Hyphen => write!(f, "-"),
            Token::Comma => write!(f, ","),
            Token::Blank(ch) => write!(f, "{}", ch),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ParseRangesError {
    NumberedFromZero,
    IndecipherableRange(Vec<Token>),
    DescendingRange,
    UnexpectedSeparator(Token),
    LexError(LexError),
}

#[derive(Debug)]
pub(crate) enum LexError {
    UnrecognizedCharacter(char),
}

impl fmt::Display for ParseRangesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseRangesError::NumberedFromZero => write!(f, "Ranges are numbered from one."),
            ParseRangesError::IndecipherableRange(tokens) => {
                let range = tokens
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join("");
                write!(f, "Indecipherable range: \"{}\"", range)
            }
            ParseRangesError::DescendingRange => write!(f, "Ranges must be ascending."),
            ParseRangesError::UnexpectedSeparator(t) => {
                write!(f, "Expected separator but found '{}'.", t)
            }
            ParseRangesError::LexError(lex_err) => match lex_err {
                LexError::UnrecognizedCharacter(ch) => {
                    write!(f, "Unrecognized character '{}'.", ch)
                }
            },
        }
    }
}

/// Parses a string into `Ranges`.
pub(crate) fn parse(s: &str) -> Result<Ranges, ParseRangesError> {
    match scan(s) {
        Result::Ok(tokens) => parse_tokens(tokens),
        Result::Err(e) => Result::Err(ParseRangesError::LexError(e)),
    }
}

/// Scans a string into a `Vec` of [`Token`]s.
fn scan(s: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.peek() {
        match ch {
            '-' => {
                tokens.push(Token::Hyphen);
                chars.next();
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            ' ' | '\t' => {
                tokens.push(Token::Blank(*ch));
                chars.next();
            }
            c if c.is_digit(10) => tokens.push(Token::Number(scan_number(&mut chars))),
            _ => return Result::Err(LexError::UnrecognizedCharacter(*ch)),
        }
    }

    Result::Ok(tokens)
}

/// Scans and consumes a number.
fn scan_number(chars: &mut Peekable<Chars>) -> usize {
    let mut number = String::new();

    while let Some(ch) = chars.peek() {
        if ch.is_digit(10) {
            number.push(*ch);
            chars.next();
        } else {
            break;
        }
    }

    // Since we've only parsed digits, we know it is a valid usize.
    number.parse::<usize>().unwrap()
}

/// Parses tokens into `Ranges`.
fn parse_tokens(tokens: Vec<Token>) -> Result<Ranges, ParseRangesError> {
    let mut cut_ranges = Vec::new();
    let mut tokens = tokens.into_iter().peekable();
    loop {
        // Parse a single range.
        let result = parse_range(&mut tokens);
        match result {
            Result::Ok(cut_range) => cut_ranges.push(cut_range),
            Result::Err(e) => return Result::Err(e),
        }
        // Consume separator.
        match tokens.peek() {
            Option::Some(Token::Comma) | Option::Some(Token::Blank(_)) => {
                tokens.next();
            }
            // No more tokens. Done parsing.
            Option::None => break,
            // Unexpected token.
            _ => {
                return Result::Err(ParseRangesError::UnexpectedSeparator(
                    tokens.next().unwrap(),
                ))
            }
        }
    }

    Result::Ok(Ranges::from_ranges(&cut_ranges))
}

/// Parse a single range from tokens.
fn parse_range<I: Iterator<Item = Token>>(
    tokens: &mut Peekable<I>,
) -> Result<CutRange, ParseRangesError> {
    // Collect all tokens until the next separator.
    let mut range = Vec::new();
    while let Some(token) = tokens.peek() {
        match token {
            Token::Hyphen | Token::Number(_) => range.push(tokens.next().unwrap()),
            _ => break,
        }
    }

    // Handle unit.
    if range.len() == 1 {
        return match range[0] {
            Token::Number(n) => match n {
                0 => Result::Err(ParseRangesError::NumberedFromZero),
                _ => Result::Ok(CutRange::Unit(n - 1)),
            },
            _ => Result::Err(ParseRangesError::IndecipherableRange(range)),
        };
    }
    // Handle "-n" and "n-".
    if range.len() == 2 {
        return match (&range[0], &range[1]) {
            (Token::Hyphen, Token::Number(end)) => match end {
                0 => Result::Err(ParseRangesError::NumberedFromZero),
                _ => Result::Ok(CutRange::FromStart(end - 1)),
            },
            (Token::Number(start), Token::Hyphen) => match start {
                0 => Result::Err(ParseRangesError::NumberedFromZero),
                _ => Result::Ok(CutRange::ToEnd(start - 1)),
            },
            _ => Result::Err(ParseRangesError::IndecipherableRange(range)),
        };
    }
    // Handle "n-m".
    if range.len() == 3 {
        return match (&range[0], &range[1], &range[2]) {
            (Token::Number(start), Token::Hyphen, Token::Number(end)) => match (start, end) {
                (0, _) | (_, 0) => Result::Err(ParseRangesError::NumberedFromZero),
                _ if start <= end => {
                    Result::Ok(CutRange::Closed(IncreasingRange::new(start - 1, end - 1)))
                }
                _ => Result::Err(ParseRangesError::DescendingRange),
            },
            _ => Result::Err(ParseRangesError::IndecipherableRange(range)),
        };
    }

    Result::Err(ParseRangesError::IndecipherableRange(range))
}

#[cfg(test)]
mod tests {
    use crate::range::{MergedRange, Ranges};

    #[test]
    fn test_parse_single_range() {
        assert_parse_ranges("1", &[MergedRange::Closed(0, 0)]);
        assert_parse_ranges("15", &[MergedRange::Closed(14, 14)]);

        assert_parse_ranges("1-1", &[MergedRange::Closed(0, 0)]);
        assert_parse_ranges("2-6", &[MergedRange::Closed(1, 5)]);
        assert_parse_ranges("9-19", &[MergedRange::Closed(8, 18)]);

        assert_parse_ranges("-1", &[MergedRange::Closed(0, 0)]);
        assert_parse_ranges("-12", &[MergedRange::Closed(0, 11)]);

        assert_parse_ranges("1-", &[MergedRange::ToEnd(0)]);
        assert_parse_ranges("23-", &[MergedRange::ToEnd(22)]);
    }

    #[test]
    fn test_parse_comma_separator() {
        use MergedRange::{Closed, ToEnd};

        assert_parse_ranges("1,2,3", &[Closed(0, 2)]);
        assert_parse_ranges("5,10,20", &[Closed(4, 4), Closed(9, 9), Closed(19, 19)]);
        assert_parse_ranges("4,8,3,2,9", &[Closed(1, 3), Closed(7, 8)]);

        assert_parse_ranges("1-2,2-4", &[Closed(0, 3)]);
        assert_parse_ranges("3-8,12-17", &[Closed(2, 7), Closed(11, 16)]);
        assert_parse_ranges("14-21,5-10,4-6", &[Closed(3, 9), Closed(13, 20)]);

        assert_parse_ranges("-3,-5", &[Closed(0, 4)]);
        assert_parse_ranges("-8,-22,-6", &[Closed(0, 21)]);

        assert_parse_ranges("5-,10-", &[ToEnd(4)]);
        assert_parse_ranges("14-,3-,6-", &[ToEnd(2)]);
    }

    #[test]
    fn test_parse_blank_separator() {
        use MergedRange::{Closed, ToEnd};

        assert_parse_ranges("1 2 3", &[Closed(0, 2)]);
        assert_parse_ranges("5\t10\t20", &[Closed(4, 4), Closed(9, 9), Closed(19, 19)]);
        assert_parse_ranges("4 8\t3\t2 9", &[Closed(1, 3), Closed(7, 8)]);

        assert_parse_ranges("1-2 2-4", &[Closed(0, 3)]);
        assert_parse_ranges("3-8\t12-17", &[Closed(2, 7), Closed(11, 16)]);
        assert_parse_ranges("14-21 5-10\t4-6", &[Closed(3, 9), Closed(13, 20)]);

        assert_parse_ranges("-3 -5", &[Closed(0, 4)]);
        assert_parse_ranges("-8\t-22 -6", &[Closed(0, 21)]);

        assert_parse_ranges("5-,10-", &[ToEnd(4)]);
        assert_parse_ranges("14- 3-\t6-", &[ToEnd(2)]);
    }

    #[test]
    fn test_parse_mixed_separator() {
        use MergedRange::{Closed, ToEnd};

        assert_parse_ranges("5,10-44 6-9\t-4", &[Closed(0, 43)]);
        assert_parse_ranges(
            "2 4\t6,8-10 -3",
            &[Closed(0, 3), Closed(5, 5), Closed(7, 9)],
        );
        assert_parse_ranges("12-34 89-,11\t10", &[Closed(9, 33), ToEnd(88)]);
        assert_parse_ranges("33-34\t9-14,-12 8", &[Closed(0, 13), Closed(32, 33)]);
        assert_parse_ranges("23-54 123- 1,2,3\t4-30 40-130", &[ToEnd(0)]);
    }

    #[test]
    fn test_parse_error() {
        // Empty string.
        assert!("".parse::<Ranges>().is_err());

        // Unknown token.
        assert!("x".parse::<Ranges>().is_err());
        assert!("!".parse::<Ranges>().is_err());
        assert!("1-#".parse::<Ranges>().is_err());
        assert!("11-22 $-5".parse::<Ranges>().is_err());

        // Indecipherable range.
        assert!("1--".parse::<Ranges>().is_err());
        assert!("23--".parse::<Ranges>().is_err());
        assert!("--45".parse::<Ranges>().is_err());
        assert!("67--89".parse::<Ranges>().is_err());
        assert!("10-11-12".parse::<Ranges>().is_err());

        // Multiple seprators.
        assert!("13,,14".parse::<Ranges>().is_err());
        assert!("15  16".parse::<Ranges>().is_err());
        assert!("17\t\t18".parse::<Ranges>().is_err());
        assert!("19, 20".parse::<Ranges>().is_err());
        assert!("21 \t22".parse::<Ranges>().is_err());
        assert!("23\t,24".parse::<Ranges>().is_err());

        // Descending range.
        assert!("9-8".parse::<Ranges>().is_err());
        assert!("123-45".parse::<Ranges>().is_err());
        assert!("1,5-3".parse::<Ranges>().is_err());
        assert!("5-7\t43-21".parse::<Ranges>().is_err());
        assert!("1,2 3\t4,98-76".parse::<Ranges>().is_err());

        //Numbered from 0.
        assert!("0,1,2".parse::<Ranges>().is_err());
        assert!("0- 3-4".parse::<Ranges>().is_err());
        assert!("-0\t4-".parse::<Ranges>().is_err());
        assert!("0-1 5,6-7\t8-".parse::<Ranges>().is_err());
    }

    fn assert_parse_ranges(input: &str, expected: &[MergedRange]) {
        let ranges: Ranges = input.parse().unwrap();
        let mut elements = ranges.elements();

        for expected_range in expected {
            assert_eq!(elements.next().unwrap(), expected_range);
        }
        assert_eq!(elements.next(), Option::None);
    }
}
