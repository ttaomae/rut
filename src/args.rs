use crate::range::Ranges;

use clap::{App, Arg, ArgGroup, ArgMatches};
use regex::Regex;

static BYTES: &str = "bytes";
static CHARACTERS: &str = "characters";
static FIELDS: &str = "fields";
static CHAR_DELIMITER: &str = "char_delimiter";
static REGEX_DELIMITER: &str = "regex_delimiter";
static OUTPUT_DELIMITER: &str = "output_delimiter";
static COMPLEMENT: &str = "complement";
static SUPPRESS: &str = "suppress";
static ZERO_TERMINATED: &str = "zero_terminated";
static NO_SPLIT: &str = "no_split";
static FILE: &str = "file";
static USAGE: &str = r"rut -b <ranges> [file]...
    rut -c <ranges> [file]...
    rut -f <ranges> [OPTIONS] [file]...";

pub(crate) struct Args {
    pub(crate) mode_args: ModeArgs,
    pub(crate) line_delimiter: u8,
    pub(crate) filenames: Vec<String>,
}

pub(crate) enum ModeArgs {
    Bytes(Ranges),
    Characters(Ranges),
    FieldsChar(Ranges, char, String, bool),
    FieldsRegex(Ranges, Regex, String, bool),
}

pub(crate) fn get_matches<'a>() -> ArgMatches<'a> {
    get_app().get_matches()
}

fn get_app<'a, 'b>() -> App<'a, 'b> {
    App::new("rut")
        .version("0.0.1")
        .usage(USAGE)
        .arg(
            Arg::with_name(BYTES)
                .short("b")
                .long("bytes")
                .allow_hyphen_values(true)
                .value_name("ranges")
                .help("Cut based on list of bytes")
                .takes_value(true)
                .display_order(0)
        )
        .arg(
            Arg::with_name(CHARACTERS)
                .short("c")
                .long("characters")
                .allow_hyphen_values(true)
                .value_name("ranges")
                .help("Cut based on a list of characters.")
                .takes_value(true)
                .display_order(1)
        )
        .arg(
            Arg::with_name(FIELDS)
                .short("f")
                .long("fields")
                .allow_hyphen_values(true)
                .value_name("ranges")
                .help("Cut based on a list of fields, assumed to be separated by a delimiter character.")
                .takes_value(true)
                .display_order(2)
        )
        .group(
            ArgGroup::with_name("modes")
                .arg(BYTES)
                .arg(CHARACTERS)
                .arg(FIELDS)
                .required(true)
        )
        .arg(
            Arg::with_name(CHAR_DELIMITER)
                .short("d")
                .long("delimiter")
                .value_name("delim")
                .help("Set the field delimiter to the character delim.")
                .takes_value(true)
                .conflicts_with_all(&[BYTES, CHARACTERS, REGEX_DELIMITER])
                .display_order(3)
        )
        .arg(
            Arg::with_name(REGEX_DELIMITER)
                .short("r")
                .long("regex-delimiter")
                .value_name("regex")
                .help("Set the field delimiter to the regular expression.")
                .takes_value(true)
                .conflicts_with_all(&[BYTES, CHARACTERS, CHAR_DELIMITER])
                .display_order(4)
        )
        .arg(
            Arg::with_name(OUTPUT_DELIMITER)
                .short("o")
                .long("output-delimiter")
                .value_name("output-delim")
                .help("Set the string used to delimit selected fields (-f).")
                .takes_value(true)
                .conflicts_with_all(&[BYTES, CHARACTERS])
                .display_order(5)
        )
        .arg(
            Arg::with_name(COMPLEMENT)
                .long("complement")
                .help("Complement the set of selected bytes, characters, or fields.")
                .takes_value(false)
                .multiple(true)
                .display_order(0)
        )
        .arg(
            Arg::with_name(SUPPRESS)
                .short("s")
                .long("only-delimited")
                .help("Suppress lines with no delimiter characters, when used with the -f option.")
                .takes_value(false)
                .multiple(true)
                .conflicts_with_all(&[BYTES, CHARACTERS])
                .display_order(1)
        )
        .arg(
            Arg::with_name(ZERO_TERMINATED)
                .short("z")
                .long("zero-terminated")
                .help("Delimits items with a zero byte rather than a newline (0x0A)")
                .multiple(true)
                .takes_value(false)
                .display_order(2)
        )
        .arg(
            Arg::with_name(NO_SPLIT)
                .short("n")
                .help("Do not split multi-byte characters. Not yet implemented (no-op).")
                .multiple(true)
                .takes_value(false)
                .conflicts_with_all(&[CHARACTERS, FIELDS])
                .display_order(3)
        )
        .arg(
            Arg::with_name(FILE)
                .index(1)
                .help("Files to cut. Files will be processed in order. Use '-' to indicate stdin.")
                .multiple(true)
                .default_value("-")
        )
}

pub(crate) fn parse_args(matches: &ArgMatches) -> Result<Args, String> {
    let complement = matches.is_present(COMPLEMENT);

    let mode_args = if let Some(ranges) = matches.value_of(BYTES) {
        ModeArgs::Bytes(validate_ranges(ranges, complement)?)
    } else if let Some(ranges) = matches.value_of(CHARACTERS) {
        ModeArgs::Characters(validate_ranges(ranges, complement)?)
    } else if let Some(ranges) = matches.value_of(FIELDS) {
        let ranges = validate_ranges(ranges, complement)?;
        let suppress = matches.is_present(SUPPRESS);
        match (
            matches.value_of(REGEX_DELIMITER),
            matches.value_of(OUTPUT_DELIMITER),
        ) {
            // Regex delimiter and joiner specified.
            (Some(regex), Some(joiner)) => {
                let delimiter = validate_regex_delimiter(regex)?;
                ModeArgs::FieldsRegex(ranges, delimiter, String::from(joiner), suppress)
            }
            // Regex delimiter specified. Use "\t" as joiner by default.
            (Some(regex), None) => {
                let delimiter = validate_regex_delimiter(regex)?;
                ModeArgs::FieldsRegex(ranges, delimiter, String::from("\t"), suppress)
            }
            // Joiner specified without regex delimiter. Use character delimiter; \t by default.
            (None, Some(output_delimiter)) => {
                let field_delimiter =
                    validate_char_delimiter(matches.value_of(CHAR_DELIMITER).unwrap_or("\t"))?;
                ModeArgs::FieldsChar(
                    ranges,
                    field_delimiter,
                    String::from(output_delimiter),
                    suppress,
                )
            }
            // No regex or joiner specified. Use character delimiter; \t by default.
            (None, None) => {
                // Use specified character delimiter, or '\t' by default.
                let field_delimiter =
                    validate_char_delimiter(matches.value_of(CHAR_DELIMITER).unwrap_or("\t"))?;
                // Use field delimiter as output delimiter.
                ModeArgs::FieldsChar(
                    ranges,
                    field_delimiter,
                    field_delimiter.to_string(),
                    suppress,
                )
            }
        }
    } else {
        // Clap should guarantee that at least one mode flag is set.
        panic!("Mode is not defined.");
    };

    let line_delimiter = if matches.is_present(ZERO_TERMINATED) {
        0
    } else {
        b'\n'
    };

    // Safe to unwrap FILE value since a default value is specified.
    let filenames = matches.values_of(FILE).unwrap().map(String::from).collect();
    Result::Ok(Args {
        mode_args,
        line_delimiter,
        filenames,
    })
}

/// Validates and returns the value as ranges, or returns an error message if validation fails.
fn validate_ranges(value: &str, complement: bool) -> Result<Ranges, String> {
    if value.is_empty() {
        return Result::Err(String::from("List of ranges must be provided"));
    }
    value
        .parse::<Ranges>()
        .map(|ranges| {
            if complement {
                ranges.complement()
            } else {
                ranges
            }
        })
        .map_err(|e| e.to_string())
}

/// Validates and returns the value as a character, or returns an error message if it is not a single character.
fn validate_char_delimiter(value: &str) -> Result<char, String> {
    let mut chars = value.chars();
    let delimiter = if let Some(ch) = chars.next() {
        ch
    } else {
        // Clap should guarantee that a value is provided.
        panic!("A delimiter was not provided.")
    };
    // Delimiter must not be more than one character.
    if chars.next().is_some() {
        return Result::Err(String::from(
            "'--delimiter <delim>' must be a single character",
        ));
    }

    Result::Ok(delimiter)
}

/// Validates and returns the value as a regular expression, or returns an error message if it is not a valid expression.
fn validate_regex_delimiter(value: &str) -> Result<Regex, String> {
    Result::Ok(Regex::new(value).map_err(|_| {
        String::from("A valid regular expression must be provided with '--regex-delimiter <regex>'")
    })?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn valid_args() {
        assert_valid_args(&["rut", "-b1"]);
        assert_valid_args(&["rut", "-b=1"]);
        assert_valid_args(&["rut", "-b", "1"]);
        assert_valid_args(&["rut", "--bytes=1"]);
        assert_valid_args(&["rut", "--bytes", "1"]);

        assert_valid_args(&["rut", "-c1"]);
        assert_valid_args(&["rut", "-c=1"]);
        assert_valid_args(&["rut", "-c", "1"]);
        assert_valid_args(&["rut", "--characters=1"]);
        assert_valid_args(&["rut", "--characters", "1"]);

        assert_valid_args(&["rut", "-f1"]);
        assert_valid_args(&["rut", "-f=1"]);
        assert_valid_args(&["rut", "-f", "1"]);
        assert_valid_args(&["rut", "--fields=1"]);
        assert_valid_args(&["rut", "--fields", "1"]);

        assert_valid_args(&["rut", "-b1", "--complement"]);
        assert_valid_args(&["rut", "-b1", "--complement", "--complement"]);
        assert_valid_args(&["rut", "-c1", "--complement"]);
        assert_valid_args(&["rut", "-c1", "--complement", "--complement"]);
        assert_valid_args(&["rut", "-f1", "--complement"]);
        assert_valid_args(&["rut", "-f1", "--complement", "--complement"]);

        assert_valid_args(&["rut", "-f1", "-d,"]);
        assert_valid_args(&["rut", "-f1", "-d=,"]);
        assert_valid_args(&["rut", "-f1", "-d", ","]);
        assert_valid_args(&["rut", "-f1", "--delimiter=,"]);
        assert_valid_args(&["rut", "-f1", "--delimiter", ","]);

        assert_valid_args(&["rut", "-f1", "-r[a-z]"]);
        assert_valid_args(&["rut", "-f1", "-r=[a-z],"]);
        assert_valid_args(&["rut", "-f1", "-r", "[a-z]"]);
        assert_valid_args(&["rut", "-f1", "--regex-delimiter=[a-z]"]);
        assert_valid_args(&["rut", "-f1", "--regex-delimiter", "[a-z]"]);

        assert_valid_args(&["rut", "-f1", "-o#"]);
        assert_valid_args(&["rut", "-f1", "-o=#"]);
        assert_valid_args(&["rut", "-f1", "-o", "#"]);
        assert_valid_args(&["rut", "-f1", "--output-delimiter=#"]);
        assert_valid_args(&["rut", "-f1", "--output-delimiter", "#"]);

        assert_valid_args(&["rut", "-f1", "-d_", "-o#"]);
        assert_valid_args(&["rut", "-f1", "-d_", "-o=#"]);
        assert_valid_args(&["rut", "-f1", "-d_", "-o", "#"]);
        assert_valid_args(&["rut", "-f1", "-d_", "--output-delimiter=#"]);
        assert_valid_args(&["rut", "-f1", "-d_", "--output-delimiter", "#"]);

        assert_valid_args(&["rut", "-f1", "-r_+", "-o#"]);
        assert_valid_args(&["rut", "-f1", "-r_+", "-o=#"]);
        assert_valid_args(&["rut", "-f1", "-r_+", "-o", "#"]);
        assert_valid_args(&["rut", "-f1", "-r_+", "--output-delimiter=#"]);
        assert_valid_args(&["rut", "-f1", "-r_+", "--output-delimiter", "#"]);

        assert_valid_args(&["rut", "-f1", "-s"]);
        assert_valid_args(&["rut", "-f1", "-ss"]);
        assert_valid_args(&["rut", "-f1", "-s", "-s"]);
        assert_valid_args(&["rut", "-f1", "--only-delimited"]);
        assert_valid_args(&["rut", "-f1", "-s", "--only-delimited"]);

        assert_valid_args(&["rut", "-b1", "-z"]);
        assert_valid_args(&["rut", "-b1", "-zz"]);
        assert_valid_args(&["rut", "-b1", "-z", "-z"]);
        assert_valid_args(&["rut", "-c1", "-z"]);
        assert_valid_args(&["rut", "-c1", "--zero-terminated"]);
        assert_valid_args(&["rut", "-c1", "-z", "--zero-terminated"]);
        assert_valid_args(&["rut", "-f1", "-z"]);
        assert_valid_args(&["rut", "-f1", "--zero-terminated", "-z"]);
        assert_valid_args(&["rut", "-f1", "-z", "-z", "-zz"]);

        assert_valid_args(&["rut", "-b1", "-n"]);
    }

    #[test]
    fn invalid_args() {
        // No arguments
        assert_invalid_args(&["rut", "-b"]);

        // Missing value
        assert_invalid_args(&["rut", "-b"]);
        assert_invalid_args(&["rut", "--bytes"]);
        assert_invalid_args(&["rut", "-c"]);
        assert_invalid_args(&["rut", "--characters"]);
        assert_invalid_args(&["rut", "-f"]);
        assert_invalid_args(&["rut", "--fields"]);

        // Invalid range.
        assert_invalid_args(&["rut", "-b", "2-1"]);
        assert_invalid_args(&["rut", "-c", "0-3"]);
        assert_invalid_args(&["rut", "-f", "0--3"]);
        assert_invalid_args(&["rut", "-b", "1,2,,3"]);
        assert_invalid_args(&["rut", "-c", "2, 3"]);
        assert_invalid_args(&["rut", "-f", "xyz"]);

        // Invalid character delimiter.
        assert_invalid_args(&["rut", "-f1", "-dfoo"]);
        assert_invalid_args(&["rut", "-f1", "--delimiter=foo"]);

        // Invalid regex delimiter.
        assert_invalid_args(&["rut", "-f1", "-r*"]);
        assert_invalid_args(&["rut", "-f1", "--regex-delimiter=*"]);
        assert_invalid_args(&["rut", "-f1", "-r(x"]);
        assert_invalid_args(&["rut", "-f1", "--regex-delimiter=(x"]);

        // Repeated arguments.
        assert_invalid_args(&["rut", "-f1", "-d,", "-d:"]);
        assert_invalid_args(&["rut", "-f1", "-d,", "--delimiter=:"]);
        assert_invalid_args(&["rut", "-f1", "-r\\d+", "-r\\s"]);
        assert_invalid_args(&["rut", "-f1", "-r\\d+", "--regex-delimiter=\\s"]);
        assert_invalid_args(&["rut", "-f1", "-o,", "-o:"]);
        assert_invalid_args(&["rut", "-f1", "-o,", "--output-delimiter:"]);
    }

    #[test]
    fn invalid_arg_combination() {
        // Multiple modes.
        assert_invalid_args(&["rut", "-b1", "-c1"]);
        assert_invalid_args(&["rut", "-b1", "-f1"]);
        assert_invalid_args(&["rut", "-c1", "-f1"]);
        assert_invalid_args(&["rut", "-b1", "-c1", "-f1"]);

        // Field mode arguments (-s, -d, -r, -o) with non-field mode.
        assert_invalid_args(&["rut", "-b1", "-s"]);
        assert_invalid_args(&["rut", "-b1", "-d,"]);
        assert_invalid_args(&["rut", "-b1", "-o#"]);
        assert_invalid_args(&["rut", "-b1", "-r_"]);
        assert_invalid_args(&["rut", "-c1", "-s"]);
        assert_invalid_args(&["rut", "-c1", "-d,"]);
        assert_invalid_args(&["rut", "-c1", "-r_"]);
        assert_invalid_args(&["rut", "-b1", "-o#"]);

        // -n with non-bytes mode.
        assert_invalid_args(&["rut", "-c1", "-n"]);
        assert_invalid_args(&["rut", "-f1", "-n"]);
    }

    fn assert_valid_args(args: &[&str]) {
        let matches = super::get_app().get_matches_from(args);
        assert!(super::parse_args(&matches).is_ok());
    }

    fn assert_invalid_args(args: &[&str]) {
        if let Ok(matches) = super::get_app().get_matches_from_safe(args) {
            assert!(super::parse_args(&matches).is_err())
        }
    }
}
