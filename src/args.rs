use crate::range::Ranges;

use clap::{App, Arg, ArgGroup, ArgMatches};
use regex::Regex;

static BYTES: &str = "bytes";
static CHARACTERS: &str = "characters";
static FIELDS: &str = "fields";
static SUPPRESS: &str = "suppress";
static CHAR_DELIMITER: &str = "delimiter";
static REGEX_DELIMITER: &str = "regex";
static JOINER: &str = "joiner";
static FILE: &str = "file";
static USAGE: &str = r"rut -b <ranges> [file]...
    rut -c <ranges> [file]...
    rut -f <ranges> [OPTIONS] [file]...";

pub(crate) struct Args {
    pub(crate) mode_args: ModeArgs,
    pub(crate) filenames: Vec<String>,
}

pub(crate) enum ModeArgs {
    Bytes(Ranges),
    Characters(Ranges),
    FieldsChar(Ranges, char, bool),
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
            Arg::with_name(JOINER)
                .short("j")
                .long("join")
                .value_name("joiner")
                .help("Set the join string for regex separated fields")
                .takes_value(true)
                .conflicts_with_all(&[BYTES, CHARACTERS])
                .display_order(5)
        )
        .arg(
            Arg::with_name(SUPPRESS)
                .short("s")
                .long("suppress")
                .help("Suppress lines with no delimiter characters, when used with the -f option.")
                .takes_value(false)
                .multiple(true)
                .conflicts_with_all(&[BYTES, CHARACTERS])
                .display_order(0)
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
    let mode_args = if let Some(ranges) = matches.value_of(BYTES) {
        ModeArgs::Bytes(validate_ranges(ranges)?)
    } else if let Some(ranges) = matches.value_of(CHARACTERS) {
        ModeArgs::Characters(validate_ranges(ranges)?)
    } else if let Some(ranges) = matches.value_of(FIELDS) {
        let ranges = validate_ranges(ranges)?;
        let suppress = matches.is_present(SUPPRESS);
        match (matches.value_of(REGEX_DELIMITER), matches.value_of(JOINER)) {
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
            // No regex or joiner specified.
            (None, None) => {
                // Use specified character delimiter, or '\t' by default.
                let delimiter =
                    validate_char_delimiter(matches.value_of(CHAR_DELIMITER).unwrap_or("\t"))?;
                ModeArgs::FieldsChar(ranges, delimiter, suppress)
            }
            // Joiner specified without regex delimiter.
            (None, Some(_)) => return Result::Err(String::from(
                "The argument '--join <joiner>' can only be used with '--regex-delimiter <regex>'",
            )),
        }
    } else {
        // Clap should guarantee that at least one mode flag is set.
        panic!("Mode is not defined.");
    };

    // Safe to unwrap FILE value since a default value is specified.
    let filenames = matches.values_of(FILE).unwrap().map(String::from).collect();
    Result::Ok(Args {
        mode_args,
        filenames,
    })
}

/// Validates and returns the value as ranges, or returns an error message if validation fails.
fn validate_ranges(value: &str) -> Result<Ranges, String> {
    if value.is_empty() {
        return Result::Err(String::from("List of ranges must be provided"));
    }
    value.parse::<Ranges>().map_err(|e| e.to_string())
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
    fn test_valid_args() {
        assert_valid_args(vec!["rut", "-b1"]);
        assert_valid_args(vec!["rut", "-b=1"]);
        assert_valid_args(vec!["rut", "-b", "1"]);
        assert_valid_args(vec!["rut", "--bytes=1"]);
        assert_valid_args(vec!["rut", "--bytes", "1"]);

        assert_valid_args(vec!["rut", "-c1"]);
        assert_valid_args(vec!["rut", "-c=1"]);
        assert_valid_args(vec!["rut", "-c", "1"]);
        assert_valid_args(vec!["rut", "--characters=1"]);
        assert_valid_args(vec!["rut", "--characters", "1"]);

        assert_valid_args(vec!["rut", "-f1"]);
        assert_valid_args(vec!["rut", "-f=1"]);
        assert_valid_args(vec!["rut", "-f", "1"]);
        assert_valid_args(vec!["rut", "--fields=1"]);
        assert_valid_args(vec!["rut", "--fields", "1"]);

        assert_valid_args(vec!["rut", "-f1", "-d,"]);
        assert_valid_args(vec!["rut", "-f1", "-d=,"]);
        assert_valid_args(vec!["rut", "-f1", "-d", ","]);
        assert_valid_args(vec!["rut", "-f1", "--delimiter=,"]);
        assert_valid_args(vec!["rut", "-f1", "--delimiter", ","]);

        assert_valid_args(vec!["rut", "-f1", "-r[a-z]"]);
        assert_valid_args(vec!["rut", "-f1", "-r=[a-z],"]);
        assert_valid_args(vec!["rut", "-f1", "-r", "[a-z]"]);
        assert_valid_args(vec!["rut", "-f1", "--regex-delimiter=[a-z]"]);
        assert_valid_args(vec!["rut", "-f1", "--regex-delimiter", "[a-z]"]);

        assert_valid_args(vec!["rut", "-f1", "-r_+", "-j#"]);
        assert_valid_args(vec!["rut", "-f1", "-r_+", "-j=#"]);
        assert_valid_args(vec!["rut", "-f1", "-r_+", "-j", "#"]);
        assert_valid_args(vec!["rut", "-f1", "-r_+", "--join=#"]);
        assert_valid_args(vec!["rut", "-f1", "-r_+", "--join", "#"]);

        assert_valid_args(vec!["rut", "-f1", "-s"]);
        assert_valid_args(vec!["rut", "-f1", "-ss"]);
        assert_valid_args(vec!["rut", "-f1", "-s", "-s"]);
        assert_valid_args(vec!["rut", "-f1", "--suppress"]);
        assert_valid_args(vec!["rut", "-f1", "-s", "--suppress"]);
    }

    #[test]
    fn test_invalid_args() {
        // No arguments
        assert_invalid_args(vec!["rut", "-b"]);

        // Missing value
        assert_invalid_args(vec!["rut", "-b"]);
        assert_invalid_args(vec!["rut", "--bytes"]);
        assert_invalid_args(vec!["rut", "-c"]);
        assert_invalid_args(vec!["rut", "--characters"]);
        assert_invalid_args(vec!["rut", "-f"]);
        assert_invalid_args(vec!["rut", "--fields"]);

        // Invalid range.
        assert_invalid_args(vec!["rut", "-b", "2-1"]);
        assert_invalid_args(vec!["rut", "-c", "0-3"]);
        assert_invalid_args(vec!["rut", "-f", "0--3"]);
        assert_invalid_args(vec!["rut", "-b", "1,2,,3"]);
        assert_invalid_args(vec!["rut", "-c", "2, 3"]);
        assert_invalid_args(vec!["rut", "-f", "xyz"]);

        // Invalid character delimiter.
        assert_invalid_args(vec!["rut", "-f1", "-dfoo"]);
        assert_invalid_args(vec!["rut", "-f1", "--delimiter=foo"]);

        // Invalid regex delimiter.
        assert_invalid_args(vec!["rut", "-f1", "-r*"]);
        assert_invalid_args(vec!["rut", "-f1", "--regex-delimiter=*"]);
        assert_invalid_args(vec!["rut", "-f1", "-r(x"]);
        assert_invalid_args(vec!["rut", "-f1", "--regex-delimiter=(x"]);

        // Repeated arguments.
        assert_invalid_args(vec!["rut", "-f1", "-d,", "-d:"]);
        assert_invalid_args(vec!["rut", "-f1", "-d,", "--delimiter=:"]);
        assert_invalid_args(vec!["rut", "-f1", "-r\\d+", "-r\\s"]);
        assert_invalid_args(vec!["rut", "-f1", "-r\\d+", "--regex-delimiter=\\s"]);
    }

    #[test]
    fn test_invalid_arg_combination() {
        // Multiple modes.
        assert_invalid_args(vec!["rut", "-b1", "-c1"]);
        assert_invalid_args(vec!["rut", "-b1", "-f1"]);
        assert_invalid_args(vec!["rut", "-c1", "-f1"]);
        assert_invalid_args(vec!["rut", "-b1", "-c1", "-f1"]);

        // Field mode arguments (-s, -d, -r, -j) with non-field mode.
        assert_invalid_args(vec!["rut", "-b1", "-s"]);
        assert_invalid_args(vec!["rut", "-b1", "-d,"]);
        assert_invalid_args(vec!["rut", "-b1", "-j#"]);
        assert_invalid_args(vec!["rut", "-b1", "-r_"]);
        assert_invalid_args(vec!["rut", "-c1", "-s"]);
        assert_invalid_args(vec!["rut", "-c1", "-d,"]);
        assert_invalid_args(vec!["rut", "-c1", "-r_"]);
        assert_invalid_args(vec!["rut", "-b1", "-j#"]);

        // -d and -r together.
        assert_invalid_args(vec!["rut", "-f1", "-d ", "-r\\s+"]);

        // -j without -r.
        assert_invalid_args(vec!["rut", "-f1", "-j!"]);
        assert_invalid_args(vec!["rut", "-f1", "-d\\t", "-j!"]);
    }

    fn assert_valid_args(args: Vec<&str>) {
        let matches = super::get_app().get_matches_from(args);
        assert!(super::parse_args(&matches).is_ok());
    }

    fn assert_invalid_args(args: Vec<&str>) {
        if let Ok(matches) = super::get_app().get_matches_from_safe(args) {
            assert!(super::parse_args(&matches).is_err())
        }
    }
}
