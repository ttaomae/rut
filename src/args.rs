use crate::range::Ranges;

use clap::{App, Arg, ArgMatches};

static BYTES: &str = "bytes";
static CHARACTERS: &str = "characters";
static FIELDS: &str = "fields";
static SUPPRESS: &str = "suppress";
static DELIMITER: &str = "delimiter";
static FILE: &str = "file";

pub(crate) struct Args {
    pub(crate) mode_args: ModeArgs,
    pub(crate) filenames: Vec<String>,
}

pub(crate) enum ModeArgs {
    Bytes(Ranges),
    Characters(Ranges),
    Fields(Ranges, char, bool),
}

pub(crate) fn get_matches<'a>() -> ArgMatches<'a> {
    get_app().get_matches()
}

fn get_app<'a, 'b>() -> App<'a, 'b> {
    App::new("rut")
        .version("0.0.1")
        .arg(
            Arg::with_name(BYTES)
                .short("b")
                .long("bytes")
                .allow_hyphen_values(true)
                .value_name("ranges")
                .help("Cut based on list of bytes")
                .takes_value(true)
                .default_value("")
        )
        .arg(
            Arg::with_name(CHARACTERS)
                .short("c")
                .long("characters")
                .allow_hyphen_values(true)
                .value_name("ranges")
                .help("Cut based on a list of characters.")
                .takes_value(true)
                .default_value("")
        )
        .arg(
            Arg::with_name(FIELDS)
                .short("f")
                .long("fields")
                .allow_hyphen_values(true)
                .value_name("ranges")
                .help("Cut based on a list of fields, assumed to be separated by a delimiter character.")
                .takes_value(true)
                .default_value("")
        )
        .arg(
            Arg::with_name(DELIMITER)
                .short("d")
                .long("delimiter")
                .value_name("delim")
                .help("Set the field delimiter to the character delim.")
                .takes_value(true)
                .multiple(true)
        )
        .arg(
            Arg::with_name(SUPPRESS)
                .short("s")
                .long("suppress")
                .help("Suppress lines with no delimiter characters, when used with the -f option.")
                .takes_value(false)
                .multiple(true)
        )
        .arg(
            Arg::with_name(FILE)
                .index(1)
                .multiple(true)
                .default_value("-")
        )
}

pub(crate) fn parse_args(matches: &ArgMatches) -> Result<Args, String> {
    // `*_mode` variables describe the number of times each mode flag appears.
    let bytes_mode = matches.occurrences_of(BYTES);
    let characters_mode = matches.occurrences_of(CHARACTERS);
    let fields_mode = matches.occurrences_of(FIELDS);

    let modes_enabled = bytes_mode + characters_mode + fields_mode;

    if modes_enabled == 0 {
        return Result::Err(String::from(
            "Must select bytes (-b), characters (-c), or fields (-f) mode.",
        ));
    }

    if modes_enabled > 1 {
        return Result::Err(String::from(
            "Only one of bytes (-b), characters (-c), or fields (-f) mode can be selected.",
        ));
    }

    // fields_mode != 1 implies that bytes or characters mode is enabled.
    if fields_mode != 1 && matches.is_present(SUPPRESS) {
        return Result::Err(String::from(
            "Supressing non-delimited lines (-s) only makes sense when operating on fields (-f).",
        ));
    }

    if fields_mode != 1 && matches.is_present(DELIMITER) {
        return Result::Err(String::from(
            "A delimiter (-d) can only be specified when operating on fields (-f).",
        ));
    }

    if matches.occurrences_of(DELIMITER) > 1 {
        return Result::Err(String::from("Delimiter (-d) must only be specified once."));
    }

    let mode_args = if bytes_mode == 1 {
        // Safe to unwrap since a default value is specified.
        let ranges = validate_ranges(matches.value_of(BYTES).unwrap())?;
        ModeArgs::Bytes(ranges)
    } else if characters_mode == 1 {
        // Safe to unwrap since a default value is specified.
        let ranges = validate_ranges(matches.value_of(CHARACTERS).unwrap())?;
        ModeArgs::Characters(ranges)
    } else if fields_mode == 1 {
        // Safe to unwrap since a default value is specified.
        let ranges = validate_ranges(matches.value_of(FIELDS).unwrap())?;
        let delimiter = validate_delimiter(matches.value_of(DELIMITER).unwrap_or("\t"))?;
        let suppress = matches.occurrences_of(DELIMITER) >= 1;
        ModeArgs::Fields(ranges, delimiter, suppress)
    } else {
        // Exactly one of the modes must be set at this point,
        // otherwise an error would be returned prior to this block.
        panic!();
    };

    // Safe to unwrap FILE value since a default value is specified.
    let filenames = matches.values_of(FILE).unwrap().map(String::from).collect();
    Result::Ok(Args {
        mode_args,
        filenames,
    })
}

fn validate_ranges(value: &str) -> Result<Ranges, String> {
    if value.is_empty() {
        return Result::Err(String::from("List of ranges must be provided."));
    }
    value.parse::<Ranges>().map_err(|e| e.to_string())
}

fn validate_delimiter(value: &str) -> Result<char, String> {
    let mut chars = value.chars();
    let mut delimiter = '\t';
    // Delimiter must be at least one character.
    if let Some(ch) = chars.next() {
        delimiter = ch;
    }
    // Delimiter must not be more than one character.
    if chars.next().is_some() {
        return Result::Err(String::from("Delimiter (-d) must be a single character."));
    }

    Result::Ok(delimiter)
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

        assert_valid_args(vec!["rut", "-f1", "-s"]);
        assert_valid_args(vec!["rut", "-f1", "-ss"]);
        assert_valid_args(vec!["rut", "-f1", "-s", "-s"]);
        assert_valid_args(vec!["rut", "-f1", "--suppress"]);
        assert_valid_args(vec!["rut", "-f1", "-s", "--suppress"]);
    }

    #[test]
    fn test_invalid_args() {
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

        // Invalid delimiter.
        assert_invalid_args(vec!["rut", "-f1", "-dfoo"]);
        assert_invalid_args(vec!["rut", "-f1", "--delimiter=foo"]);

        // Repeated arguments.
        assert_invalid_args(vec!["rut", "-f1", "-d,", "-d:"]);
        assert_invalid_args(vec!["rut", "-f1", "-d,", "--delimiter=:"]);
    }

    #[test]
    fn test_invalid_arg_combination() {
        // -s and -d with
        assert_invalid_args(vec!["rut", "-b1", "-s"]);
        assert_invalid_args(vec!["rut", "-b1", "-d,"]);
        assert_invalid_args(vec!["rut", "-c1", "-s"]);
        assert_invalid_args(vec!["rut", "-c1", "-d,"]);
    }

    fn assert_valid_args(args: Vec<&str>) {
        let matches = super::get_app().get_matches_from(args);
        assert!(super::parse_args(&matches).is_ok());
    }

    fn assert_invalid_args(args: Vec<&str>) {
        let matches = super::get_app().get_matches_from(args);
        let result = super::parse_args(&matches);
        // dbg!(&result);
        assert!(result.is_err());
    }
}
