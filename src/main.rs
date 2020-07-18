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
        Result::Ok(args) => cut(args),
        Result::Err(err) => {
            eprintln!("error: {}\n", err);
            eprintln!("{}", matches.usage());
            std::process::exit(1);
        }
    }
}

fn cut(args: Args) {
    let filenames = args.filenames;

    let mut stdout = std::io::stdout();

    match args.mode_args {
        ModeArgs::Bytes(ranges) => for_each_file(filenames, |mut file| {
            cut::cut_bytes(&mut file, &mut stdout, &ranges)
        }),
        ModeArgs::Characters(ranges) => for_each_file(filenames, |mut file| {
            cut::cut_characters(&mut file, &mut stdout, &ranges)
        }),
        ModeArgs::FieldsChar(ranges, delimiter, suppress) => {
            for_each_file(filenames, |mut file| {
                cut::cut_fields_with_char(&mut file, &mut stdout, delimiter, suppress, &ranges)
            })
        }
        ModeArgs::FieldsRegex(ranges, delimiter, joiner, suppress) => {
            for_each_file(filenames, |mut file| {
                cut::cut_fields_with_regex(
                    &mut file,
                    &mut stdout,
                    &delimiter,
                    &joiner,
                    suppress,
                    &ranges,
                )
            })
        }
    }
}

fn for_each_file<F>(filenames: Vec<String>, mut f: F)
where
    F: FnMut(Box<dyn Read>) -> std::io::Result<()>,
{
    for filename in filenames {
        let file: Box<dyn Read> = if filename == "-" {
            Box::new(std::io::stdin())
        } else {
            match File::open(&filename) {
                Result::Ok(file) => Box::new(file),
                Result::Err(err) => {
                    eprintln!("{}: {}", &filename, err);
                    continue;
                }
            }
        };

        if let std::io::Result::Err(err) = f(file) {
            eprintln!("{}: {}", &filename, err);
        }
    }
}
