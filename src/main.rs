mod args;
mod cut;
mod range;

use args::{Args, ModeArgs};
use std::fs::File;
use std::io::Read;
use std::result::Result;

fn main() {
    let matches = args::get_matches();

    match args::parse_args(&matches) {
        Result::Ok(args) => {
            if cut(args).is_err() {
                print_error_and_exit("", matches.usage(), 1)
            }
        }
        Result::Err(err) => print_error_and_exit(&err, matches.usage(), 1),
    }
}

fn print_error_and_exit(error: &str, usage: &str, code: i32) {
    if !error.is_empty() {
        eprintln!("error: {}\n", error);
    }
    eprintln!("{}", usage);
    std::process::exit(code);
}

fn cut(args: Args) -> Result<(), ()> {
    let filenames = args.filenames;
    let line_delimiter = args.line_delimiter;

    let mut stdout = std::io::stdout();

    match args.mode_args {
        ModeArgs::Bytes(ranges) => for_each_file(filenames, |mut file| {
            cut::cut_bytes(&mut file, &mut stdout, line_delimiter, &ranges)
        }),
        ModeArgs::Characters(ranges) => for_each_file(filenames, |mut file| {
            cut::cut_characters(&mut file, &mut stdout, line_delimiter, &ranges)
        }),
        ModeArgs::FieldsChar(ranges, field_delimiter, output_delimiter, suppress) => {
            for_each_file(filenames, |mut file| {
                cut::cut_fields_with_char(
                    &mut file,
                    &mut stdout,
                    line_delimiter,
                    field_delimiter,
                    &output_delimiter,
                    suppress,
                    &ranges,
                )
            })
        }
        ModeArgs::FieldsRegex(ranges, delimiter, joiner, suppress) => {
            for_each_file(filenames, |mut file| {
                cut::cut_fields_with_regex(
                    &mut file,
                    &mut stdout,
                    line_delimiter,
                    &delimiter,
                    &joiner,
                    suppress,
                    &ranges,
                )
            })
        }
    }
}

fn for_each_file<F>(filenames: Vec<String>, mut f: F) -> Result<(), ()>
where
    F: FnMut(Box<dyn Read>) -> std::io::Result<()>,
{
    let mut error = false;
    for filename in filenames {
        let file: Box<dyn Read> = if filename == "-" {
            Box::new(std::io::stdin())
        } else {
            match File::open(&filename) {
                Result::Ok(file) => Box::new(file),
                Result::Err(err) => {
                    error = true;
                    eprintln!("{}: {}", &filename, err);
                    continue;
                }
            }
        };

        if let std::io::Result::Err(err) = f(file) {
            error = true;
            eprintln!("{}: {}", &filename, err);
        }
    }

    if error {
        Result::Err(())
    } else {
        Result::Ok(())
    }
}
