use crate::range::{MergedRange, Ranges};
use regex::Regex;
use std::fmt::Debug;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::result::Result;
use std::vec::Vec;

enum Suppress {
    /// Not applicable.
    NA,
    /// Do not output lines with no delimiter.
    On,
    /// Output entire line if it contains no delimiter.
    Off,
}

/// Selects bytes from the input, based on the specified ranges, and writes it to the output.
pub(crate) fn cut_bytes<R, W>(input: &mut R, output: &mut W, ranges: &Ranges) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    cut_any(
        input,
        output,
        b'\n',
        |bytes| Result::Ok(bytes.to_vec()),
        Suppress::NA,
        |bytes| bytes,
        ranges,
    )
}

/// Selects characters from the input, based on the specified ranges, and writes it to the output.
pub(crate) fn cut_characters<R, W>(input: &mut R, output: &mut W, ranges: &Ranges) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    cut_any(
        input,
        output,
        b'\n',
        // Convert to chars.
        |bytes| {
            let result = bytes.clone();
            match String::from_utf8(result) {
                Ok(string) => Result::Ok(string.chars().collect()),
                Err(_) => Result::Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Input was not valid UTF-8",
                )),
            }
        },
        Suppress::NA,
        |chars| {
            chars
                .iter()
                .flat_map(|ch| {
                    let mut buf = vec![0u8; 4];
                    let len = ch.encode_utf8(&mut buf).len();
                    buf.truncate(len);
                    buf
                })
                .collect()
        },
        ranges,
    )
}

/// Splits and selects fields separated by a delimiter character. Rejoins fields using the delimiter
/// then writes the selected fields to the output.
pub(crate) fn cut_fields_with_char<R, W>(
    input: &mut R,
    output: &mut W,
    delimiter: char,
    suppress: bool,
    ranges: &Ranges,
) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    cut_any(
        input,
        output,
        b'\n',
        |bytes| {
            let line = String::from_utf8(bytes.to_vec()).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Input was not valid UTF-8")
            })?;
            let fields: Vec<String> = line.split(delimiter).map(|s| s.to_string()).collect();
            Result::Ok(fields)
        },
        if suppress {
            Suppress::On
        } else {
            Suppress::Off
        },
        |fields| fields.join(&delimiter.to_string()).as_bytes().to_vec(),
        &ranges,
    )
}

/// Splits and selects fields separated by regex delimiter. Rejoins fields using a specified
/// "joiner" string then writes the selected fields to the output.
pub(crate) fn cut_fields_with_regex<R, W>(
    input: &mut R,
    output: &mut W,
    delimiter: &Regex,
    joiner: &str,
    suppress: bool,
    ranges: &Ranges,
) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    cut_any(
        input,
        output,
        b'\n',
        |bytes| {
            let line = String::from_utf8(bytes.to_vec()).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Input was not valid UTF-8")
            })?;
            let fields: Vec<String> = delimiter.split(&line).map(|s| s.to_string()).collect();
            Result::Ok(fields)
        },
        if suppress {
            Suppress::On
        } else {
            Suppress::Off
        },
        |fields| fields.join(&joiner).as_bytes().to_vec(),
        ranges,
    )
}

fn cut_any<R, W, S, T, J>(
    input: &mut R,
    output: &mut W,
    line_delimiter: u8,
    split: S,
    suppress: Suppress,
    join: J,
    ranges: &Ranges,
) -> io::Result<()>
where
    R: Read,
    W: Write,
    S: Fn(&Vec<u8>) -> Result<Vec<T>, io::Error>,
    T: Clone + Debug,
    J: Fn(Vec<T>) -> Vec<u8>,
{
    let mut reader = BufReader::new(input);
    let mut buf = Vec::new();
    while reader.read_until(line_delimiter, &mut buf)? > 0 {
        if buf.ends_with(&[line_delimiter]) {
            buf.pop();
        }
        // Split the line into elements.
        let elements = split(&buf)?;

        // Clear buffer for next line.
        buf.clear();

        match suppress {
            // No special handling.
            Suppress::NA => {
                let cut_elements = select(&elements, &ranges);
                let mut line = join(cut_elements);
                line.push(line_delimiter);
                output.write_all(&line)?;
            }
            // For cases which must handle suppression, a single element after splitting means
            // there was no delimiter. Applies to Suppress::On and Supress::Off.
            Suppress::On => {
                // Only output lines which contain a delimiter.
                if elements.len() != 1 {
                    let cut_elements = select(&elements, &ranges);
                    let mut line = join(cut_elements);
                    line.push(line_delimiter);
                    output.write_all(&line)?;
                }
            }
            Suppress::Off => {
                // Output entire line if there is no delimiter.
                let mut line = if elements.len() == 1 {
                    join(elements)
                } else {
                    let cut_elements = select(&elements, &ranges);
                    join(cut_elements)
                };
                line.push(line_delimiter);
                output.write_all(&line)?;
            }
        }
    }
    Result::Ok(())
}

/// Selects the specified ranges from the input slice and returns a `Vec` with those elements.
fn select<T: Clone + Debug>(input: &[T], ranges: &Ranges) -> Vec<T> {
    let mut result = Vec::new();

    for range in ranges.elements() {
        match range {
            MergedRange::Closed(s, e) => {
                // Range is outside size of input and since ranges are sorted, all following ranges
                // will also be outside input. Skip and break from loop.
                if *s >= input.len() {
                    break;
                }

                // Otherwise include elements in range, up to the last element.
                let end = std::cmp::min(*e + 1, input.len());
                result.extend_from_slice(&input[*s..end]);
            }
            MergedRange::ToEnd(s) => {
                // Range is outside size of input and since ranges are sorted, all following ranges
                // will also be outside input. Skip and break from loop.
                if *s >= input.len() {
                    break;
                }

                // Otherwise include the remaining elements.
                result.extend_from_slice(&input[*s..])
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use crate::range::Ranges;
    use regex::Regex;

    #[test]
    fn test_cut_bytes() {
        // One line.
        let input = &[1, 2, 3, 4, 5, 6, 7, 8];
        assert_cut_bytes(
            &mut input.clone(),
            "1-",
            vec![1, 2, 3, 4, 5, 6, 7, 8, b'\n'],
        );
        assert_cut_bytes(&mut input.clone(), "2-5", vec![2, 3, 4, 5, b'\n']);
        assert_cut_bytes(&mut input.clone(), "-3,6-", vec![1, 2, 3, 6, 7, 8, b'\n']);
        assert_cut_bytes(&mut input.clone(), "1,2,4,8,16-", vec![1, 2, 4, 8, b'\n']);

        // Multiple lines.
        let input = &[
            1, 2, 3, 4, 5, 6, 7, 8, b'\n', 11, 12, 13, 14, 15, 16, 17, 18,
        ];
        assert_cut_bytes(
            &mut input.clone(),
            "1-",
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, b'\n', 11, 12, 13, 14, 15, 16, 17, 18, b'\n',
            ],
        );
        assert_cut_bytes(
            &mut input.clone(),
            "2-4,7-",
            vec![2, 3, 4, 7, 8, b'\n', 12, 13, 14, 17, 18, b'\n'],
        );
        assert_cut_bytes(
            &mut input.clone(),
            "4-8",
            vec![4, 5, 6, 7, 8, b'\n', 14, 15, 16, 17, 18, b'\n'],
        );

        assert_cut_bytes(
            &[1, 2, b'\n', 3, 4][..],
            "1-",
            vec![1, 2, b'\n', 3, 4, b'\n'],
        );
        assert_cut_bytes(
            &[1, 2, b'\n', 3, 4, b'\n'][..],
            "1-",
            vec![1, 2, b'\n', 3, 4, b'\n'],
        );
        assert_cut_bytes(
            &[1, 2, b'\n', b'\n', 3, 4, b'\n'][..],
            "1-",
            vec![1, 2, b'\n', b'\n', 3, 4, b'\n'],
        );

        // Different sized lines.
        let input = &[1, 2, 3, 4, 5, 6, 7, 8, b'\n', 11, 12, 13, 14, 15, 16];
        assert_cut_bytes(
            &mut input.clone(),
            "1-",
            vec![1, 2, 3, 4, 5, 6, 7, 8, b'\n', 11, 12, 13, 14, 15, 16, b'\n'],
        );
        assert_cut_bytes(
            &mut input.clone(),
            "5-",
            vec![5, 6, 7, 8, b'\n', 15, 16, b'\n'],
        );
        assert_cut_bytes(
            &mut input.clone(),
            "2-4,7-9",
            vec![2, 3, 4, 7, 8, b'\n', 12, 13, 14, b'\n'],
        );

        // Many different sized lines.
        let input = &[
            1, b'\n', 11, 12, b'\n', 21, 22, 23, b'\n', 31, 32, 33, 34, b'\n', 41, 42, 43, 44, 45,
        ];
        assert_cut_bytes(
            &mut input.clone(),
            "1-",
            vec![
                1, b'\n', 11, 12, b'\n', 21, 22, 23, b'\n', 31, 32, 33, 34, b'\n', 41, 42, 43, 44,
                45, b'\n',
            ],
        );
        assert_cut_bytes(
            &mut input.clone(),
            "3,5-",
            vec![b'\n', b'\n', 23, b'\n', 33, b'\n', 43, 45, b'\n'],
        );

        // Non-UTF-8.
        let input = &[255, 254, 253, b'\n', 252, 251, 250];
        assert_cut_bytes(
            &mut input.clone(),
            "1-",
            vec![255, 254, 253, b'\n', 252, 251, 250, b'\n'],
        );
        assert_cut_bytes(&mut input.clone(), "2", vec![254, b'\n', 251, b'\n']);
    }

    #[test]
    fn test_cut_bytes_trailing_newline() {
        assert_cut_bytes(&[], "1-", vec![]);
        assert_cut_bytes(&[b'\n'], "1-", vec![b'\n']);

        assert_cut_bytes(&[1, 2, 3, 4], "1-", vec![1, 2, 3, 4, b'\n']);
        assert_cut_bytes(&[1, 2, 3, 4], "1,3", vec![1, 3, b'\n']);
        assert_cut_bytes(&[1, 2, 3, 4, b'\n'], "1-", vec![1, 2, 3, 4, b'\n']);
        assert_cut_bytes(&[1, 2, 3, 4, b'\n'], "2,4", vec![2, 4, b'\n']);

        assert_cut_bytes(&[1, 2, 3, 4, b'\n', 5, 6, 7, 8], "1-", vec![1, 2, 3, 4, b'\n', 5, 6, 7, 8, b'\n']);
        assert_cut_bytes(&[1, 2, 3, 4, b'\n', 5, 6, 7, 8], "1,3", vec![1, 3, b'\n', 5, 7, b'\n']);
        assert_cut_bytes(&[1, 2, 3, 4, b'\n', 5, 6, 7, 8, b'\n'], "1-", vec![1, 2, 3, 4, b'\n', 5, 6, 7, 8, b'\n']);
        assert_cut_bytes(&[1, 2, 3, 4, b'\n', 5, 6, 7, 8, b'\n'], "2,4", vec![2, 4, b'\n', 6, 8, b'\n']);
    }

    fn assert_cut_bytes(mut input: &[u8], ranges: &str, expected: Vec<u8>) {
        let mut output = Vec::new();
        let ranges: Ranges = ranges.parse().unwrap();
        super::cut_bytes(&mut input, &mut output, &ranges).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn test_cut_characters() {
        // Empty.
        assert_cut_chars("", "1-", "");
        assert_cut_chars("\n", "1-", "\n");

        // One line.
        assert_cut_chars("abcdefghi", "1-", "abcdefghi\n");
        assert_cut_chars("abcdefghi", "1,3,5", "ace\n");
        assert_cut_chars("abcdefghi", "-2,8-", "abhi\n");
        assert_cut_chars("abcdefghi", "1,4-6,9", "adefi\n");
        assert_cut_chars("abcdefghi", "1-5,10-20", "abcde\n");

        // Multi-byte characters.
        assert_cut_chars("αβγδεζηθ", "1-", "αβγδεζηθ\n");
        assert_cut_chars("αβγδεζηθ", "1,3,5", "αγε\n");
        assert_cut_chars("αβγδεζηθ", "2-4,8", "βγδθ\n");
        assert_cut_chars("αβγδεζηθ", "1-2,7-8", "αβηθ\n");

        assert_cut_chars("αaβbγgδdεe", "1-", "αaβbγgδdεe\n");
        assert_cut_chars("αaβbγgδdεe", "1-3,7-9", "αaβδdε\n");
        assert_cut_chars("αaβbγgδdεe", "2,4-6,8,10-12", "abγgde\n");
        assert_cut_chars("αaβbγgδdεe", "1,5-7,10", "αγgδe\n");

        // Multiple lines.
        assert_cut_chars("abcdefghi\njklmnopqr", "1-", "abcdefghi\njklmnopqr\n");
        assert_cut_chars("abcdefghi\njklmnopqr\n", "1-", "abcdefghi\njklmnopqr\n");
        assert_cut_chars("abcdefghi\n\njklmnopqr\n", "1-", "abcdefghi\n\njklmnopqr\n");
        assert_cut_chars(
            "abcdefghi\n\njklmnopqr\n\n",
            "1-",
            "abcdefghi\n\njklmnopqr\n\n",
        );
        assert_cut_chars("abcdefghi\njklmnopqr", "4-7", "defg\nmnop\n");
        assert_cut_chars("abcdefghi\njklmnopqr", "-3,8-", "abchi\njklqr\n");

        // Multi-byte characters; different sized lines.
        assert_cut_chars("αaβbγgδdε\neζzηi", "1-", "αaβbγgδdε\neζzηi\n");
        assert_cut_chars("αaβbγgδdε\neζzηi", "4-", "bγgδdε\nηi\n");
        assert_cut_chars("αaβbγgδdε\neζzηi", "2,7-", "aδdε\nζ\n");
        assert_cut_chars("αaβbγgδdε\neζzηi", "8-", "dε\n\n");
        assert_cut_chars("αaβbγgδdε\neζzηi", "2,4-5,9", "abγε\nζηi\n");

        // Many different sized lines.
        assert_cut_chars(
            "a\nbc\ndef\nghij\nklmno\npqrstu\nvwyxzzz",
            "1-",
            "a\nbc\ndef\nghij\nklmno\npqrstu\nvwyxzzz\n",
        );
        assert_cut_chars(
            "a\nbc\ndef\nghij\nklmno\npqrstu\nvwyxzzz",
            "3,5-",
            "\n\nf\ni\nmo\nrtu\nyzzz\n",
        );
    }

    fn assert_cut_chars(input: &str, ranges: &str, expected: &str) {
        let mut output = Vec::new();
        let ranges: Ranges = ranges.parse().unwrap();
        super::cut_characters(&mut input.as_bytes(), &mut output, &ranges).unwrap();
        let actual = String::from_utf8(output).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cut_fields_with_char() {
        // Empty.
        assert_cut_fields_with_char("", "1-", ' ', false, "");
        assert_cut_fields_with_char("\n", "1-", ' ', false, "\n");

        // One line.
        assert_cut_fields_with_char("a b c d e f", "1-", ' ', false, "a b c d e f\n");
        assert_cut_fields_with_char("a b c d e f   ", "1-", ' ', false, "a b c d e f   \n");
        assert_cut_fields_with_char("a b c d e f", "1,3,5", ' ', false, "a c e\n");
        assert_cut_fields_with_char("a b c d e f", "2-4,6-", ' ', false, "b c d f\n");
        assert_cut_fields_with_char("a b c d e f    ", "6-", ' ', false, "f    \n");
        assert_cut_fields_with_char("a b c d e f    ", "7-", ' ', false, "   \n");

        assert_cut_fields_with_char("abc def ghi jkl", "1-", ' ', false, "abc def ghi jkl\n");
        assert_cut_fields_with_char("abc def ghi jkl", "2,4", ' ', false, "def jkl\n");
        assert_cut_fields_with_char("abc def ghi jkl", "3-", ' ', false, "ghi jkl\n");

        // Multiple lines.
        assert_cut_fields_with_char(
            "a bc def gh\ni jk lmn o pq",
            "1-",
            ' ',
            false,
            "a bc def gh\ni jk lmn o pq\n",
        );
        assert_cut_fields_with_char(
            "a bc def gh\ni jk lmn o pq",
            "1,3,5",
            ' ',
            false,
            "a def\ni lmn pq\n",
        );
        assert_cut_fields_with_char("a bc def gh\ni jk lmn o pq", "4-", ' ', false, "gh\no pq\n");

        // Multi-byte characters.
        assert_cut_fields_with_char(
            "αa β bγg δ\nd εeζz ηi",
            "1-",
            ' ',
            false,
            "αa β bγg δ\nd εeζz ηi\n",
        );
        assert_cut_fields_with_char("αa β bγg δ\nd εeζz ηi", "1,2", ' ', false, "αa β\nd εeζz\n");
        assert_cut_fields_with_char(
            "αa β bγg δ\nd εeζz ηi",
            "1,3-",
            ' ',
            false,
            "αa bγg δ\nd ηi\n",
        );

        // Many different sized lines.
        assert_cut_fields_with_char(
            "a b\nc d e\nf gh ijkl\nm n o p q r s\ntuv wx   y z",
            "1-",
            ' ',
            false,
            "a b\nc d e\nf gh ijkl\nm n o p q r s\ntuv wx   y z\n",
        );
        assert_cut_fields_with_char(
            "a b\nc d e\nf gh ijkl\nm n o p q r s\ntuv wx   y z",
            "1,4",
            ' ',
            false,
            "a\nc\nf\nm p\ntuv \n",
        );
        assert_cut_fields_with_char(
            "a b\nc d e\nf gh ijkl\nm n o p q r s\ntuv wx   y z",
            "3-",
            ' ',
            false,
            "\ne\nijkl\no p q r s\n  y z\n",
        );
    }

    #[test]
    fn test_cut_fields_with_char_suppress() {
        // Single line. No suppress.
        assert_cut_fields_with_char("a b c", "1-", ' ', false, "a b c\n");
        assert_cut_fields_with_char("a b c", "1-", ' ', true, "a b c\n");
        assert_cut_fields_with_char("a b c", "2", ' ', false, "b\n");
        assert_cut_fields_with_char("a b c", "2", ' ', true, "b\n");

        // Single line. Suppress.
        assert_cut_fields_with_char("abc", "1-", ' ', false, "abc\n");
        assert_cut_fields_with_char("abc", "1-", ' ', true, "");
        assert_cut_fields_with_char("abc", "4", ' ', false, "abc\n");
        assert_cut_fields_with_char("abc", "4", ' ', true, "");

        // Multiple lines. No suppressed lines.
        assert_cut_fields_with_char(
            "a b c\nd e f\ng h i",
            "1-",
            ' ',
            false,
            "a b c\nd e f\ng h i\n",
        );
        assert_cut_fields_with_char(
            "a b c\nd e f\ng h i",
            "1-",
            ' ',
            true,
            "a b c\nd e f\ng h i\n",
        );
        assert_cut_fields_with_char("a b c\nd e f\ng h i", "4-", ' ', false, "\n\n\n");
        assert_cut_fields_with_char("a b c\nd e f\ng h i", "4-", ' ', true, "\n\n\n");

        // Multiple lines. With suppressed lines.
        assert_cut_fields_with_char("a b c\ndef\ng h i", "1-", ' ', false, "a b c\ndef\ng h i\n");
        assert_cut_fields_with_char("a b c\ndef\ng h i", "1-", ' ', true, "a b c\ng h i\n");
        assert_cut_fields_with_char("a b c\ndef\ng h i", "2", ' ', false, "b\ndef\nh\n");
        assert_cut_fields_with_char("a b c\ndef\ng h i", "2", ' ', true, "b\nh\n");

        assert_cut_fields_with_char("abc\nd e f\nghi", "1-", ' ', false, "abc\nd e f\nghi\n");
        assert_cut_fields_with_char("abc\nd e f\nghi", "1-", ' ', true, "d e f\n");
        assert_cut_fields_with_char("abc\nd e f\nghi", "3", ' ', false, "abc\nf\nghi\n");
        assert_cut_fields_with_char("abc\nd e f\nghi", "3", ' ', true, "f\n");
    }

    #[test]
    fn test_cut_fields_with_char_delimiter() {
        // Single byte delimiter.
        assert_cut_fields_with_char("a b c d e", "1-", ' ', false, "a b c d e\n");
        assert_cut_fields_with_char("a b c d e", "2,4", ' ', false, "b d\n");
        assert_cut_fields_with_char("a:b:c:d:e", "1-", ':', false, "a:b:c:d:e\n");
        assert_cut_fields_with_char("a:b:c:d:e", "2,4", ':', false, "b:d\n");
        assert_cut_fields_with_char("a_b_c_d_e", "1-", '_', false, "a_b_c_d_e\n");
        assert_cut_fields_with_char("a_b_c_d_e", "2,4", '_', false, "b_d\n");

        // Multi-byte delimiter.
        assert_cut_fields_with_char("a→b→c→d→e", "1-", '→', false, "a→b→c→d→e\n");
        assert_cut_fields_with_char("a→b→c→d→e", "2,4", '→', false, "b→d\n");
        assert_cut_fields_with_char("a⭐b⭐c⭐d⭐e", "1-", '⭐', false, "a⭐b⭐c⭐d⭐e\n");
        assert_cut_fields_with_char("a⭐b⭐c⭐d⭐e", "2,4", '⭐', false, "b⭐d\n");
    }

    fn assert_cut_fields_with_char(
        input: &str,
        ranges: &str,
        delimiter: char,
        suppress: bool,
        expected: &str,
    ) {
        let mut output = Vec::new();
        let ranges: Ranges = ranges.parse().unwrap();
        super::cut_fields_with_char(
            &mut input.as_bytes(),
            &mut output,
            delimiter,
            suppress,
            &ranges,
        )
        .unwrap();
        let actual = String::from_utf8(output).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cut_fields_with_regex() {
        // Empty.
        assert_cut_fields_with_regex("", "1-", r"\s+", "\t", false, "");
        assert_cut_fields_with_regex("\n", "1-", r"\s+", "\t", false, "\n");

        // One line.
        assert_cut_fields_with_regex(
            "a b\tc  d\t\te \tf \t\t   g",
            "1-",
            r"\s+",
            "\t",
            false,
            "a\tb\tc\td\te\tf\tg\n",
        );
        assert_cut_fields_with_regex(
            "a b\tc  d\t\te \tf \t\t   g",
            "2,4,6",
            r"\s+",
            "\t",
            false,
            "b\td\tf\n",
        );
        assert_cut_fields_with_regex(
            "a b\tc  d\t\te \tf \t\t   g",
            "-2,5,9-",
            r"\s+",
            "\t",
            false,
            "a\tb\te\n",
        );

        assert_cut_fields_with_regex(
            "abc def\tghi  jkl\t mno",
            "1-",
            r"\s+",
            "\t",
            false,
            "abc\tdef\tghi\tjkl\tmno\n",
        );
        assert_cut_fields_with_regex(
            "abc def\tghi  jkl\t mno",
            "1,4-",
            r"\s+",
            "\t",
            false,
            "abc\tjkl\tmno\n",
        );

        // Multiple lines.
        assert_cut_fields_with_regex(
            "a b c\na\tb\tc\na  b\t\tc",
            "1-",
            r"\s+",
            "\t",
            false,
            "a\tb\tc\na\tb\tc\na\tb\tc\n",
        );
        assert_cut_fields_with_regex(
            "a b c\na\tb\tc\na  b\t\tc",
            "1,3",
            r"\s+",
            "\t",
            false,
            "a\tc\na\tc\na\tc\n",
        );
        assert_cut_fields_with_regex(
            "a b c\na\tb\tc\na  b\t\tc",
            "2,4",
            r"\s+",
            "\t",
            false,
            "b\nb\nb\n",
        );

        // Multi-byte characters.
        assert_cut_fields_with_regex(
            "αa β\tbγg  δ\nd\t\tεeζz \t ηi",
            "1-",
            r"\s+",
            "\t",
            false,
            "αa\tβ\tbγg\tδ\nd\tεeζz\tηi\n",
        );
        assert_cut_fields_with_regex(
            "αa β\tbγg  δ\nd\t\tεeζz \t ηi",
            "3-",
            r"\s+",
            "\t",
            false,
            "bγg\tδ\nηi\n",
        );
        assert_cut_fields_with_regex(
            "αa β\tbγg  δ\nd\t\tεeζz \t ηi",
            "1,4",
            r"\s+",
            "\t",
            false,
            "αa\tδ\nd\n",
        );

        // Many different sized lines.
        assert_cut_fields_with_regex(
            "a b c\nd  e   f g\nh\ti\nj \tk\t l m n o p\nq\tr\ts t\nu v\nw\tx y\tz",
            "1-",
            r"\s+",
            "\t",
            false,
            "a\tb\tc\nd\te\tf\tg\nh\ti\nj\tk\tl\tm\tn\to\tp\nq\tr\ts\tt\nu\tv\nw\tx\ty\tz\n",
        );
        assert_cut_fields_with_regex(
            "a b c\nd  e   f g\nh\ti\nj \tk\t l m n o p\nq\tr\ts t\nu v\nw\tx y\tz",
            "2,4,6",
            r"\s+",
            "\t",
            false,
            "b\ne\tg\ni\nk\tm\to\nr\tt\nv\nx\tz\n",
        );
        assert_cut_fields_with_regex(
            "a b c\nd  e   f g\nh\ti\nj \tk\t l m n o p\nq\tr\ts t\nu v\nw\tx y\tz",
            "1,5-",
            r"\s+",
            "\t",
            false,
            "a\nd\nh\nj\tn\to\tp\nq\nu\nw\n",
        );
    }

    #[test]
    fn test_cut_fields_with_regex_suppress() {
        // Single line. No suppress.
        assert_cut_fields_with_regex("a b\tc", "1-", r"\s+", "\t", false, "a\tb\tc\n");
        assert_cut_fields_with_regex("a b\tc", "1-", r"\s+", "\t", true, "a\tb\tc\n");
        assert_cut_fields_with_regex("a b\tc", "2", r"\s+", "\t", false, "b\n");
        assert_cut_fields_with_regex("a b\tc", "2", r"\s+", "\t", true, "b\n");

        // Single line. With suppress.
        assert_cut_fields_with_regex("a:b:c", "1-", r"\s+", "\t", false, "a:b:c\n");
        assert_cut_fields_with_regex("a:b:c", "1-", r"\s+", "\t", true, "");
        assert_cut_fields_with_regex("a:b:c", "3", r"\s+", "\t", false, "a:b:c\n");
        assert_cut_fields_with_regex("a:b:c", "3", r"\s+", "\t", true, "");

        // Multiple lines. No suppressed lines.
        assert_cut_fields_with_regex(
            "a b c\nd\te\tf\ng h\ti\n",
            "1-",
            r"\s+",
            "\t",
            false,
            "a\tb\tc\nd\te\tf\ng\th\ti\n",
        );
        assert_cut_fields_with_regex(
            "a b c\nd\te\tf\ng h\ti\n",
            "1-",
            r"\s+",
            "\t",
            true,
            "a\tb\tc\nd\te\tf\ng\th\ti\n",
        );
        assert_cut_fields_with_regex(
            "a b c\nd\te\tf\ng h\ti\n",
            "4-",
            r"\s+",
            "\t",
            false,
            "\n\n\n",
        );
        assert_cut_fields_with_regex(
            "a b c\nd\te\tf\ng h\ti\n",
            "4-",
            r"\s+",
            "\t",
            true,
            "\n\n\n",
        );

        // Multiple lines. With suppressed lines.
        assert_cut_fields_with_regex(
            "a b c\ndef\ng\th\ti",
            "1-",
            r"\s+",
            "\t",
            false,
            "a\tb\tc\ndef\ng\th\ti\n",
        );
        assert_cut_fields_with_regex(
            "a b c\ndef\ng\th\ti",
            "1-",
            r"\s+",
            "\t",
            true,
            "a\tb\tc\ng\th\ti\n",
        );
        assert_cut_fields_with_regex(
            "a b c\ndef\ng\th\ti",
            "1",
            r"\s+",
            "\t",
            false,
            "a\ndef\ng\n",
        );
        assert_cut_fields_with_regex("a b c\ndef\ng\th\ti", "1", r"\s+", "\t", true, "a\ng\n");
    }

    #[test]
    fn test_cut_fields_with_regex_delimiter() {
        // Single character delimiter
        assert_cut_fields_with_regex("abc.def.ghi", "1-", r"\.", "-", false, "abc-def-ghi\n");
        assert_cut_fields_with_regex("abc.def.ghi", "1,3", r"\.", "_", false, "abc_ghi\n");
        assert_cut_fields_with_regex("ab1cd2ef3gh", "1-", r"\d", " ", false, "ab cd ef gh\n");
        assert_cut_fields_with_regex("ab1cd2ef3gh", "2,4", r"\d", "#", false, "cd#gh\n");
        assert_cut_fields_with_regex("12x34y56z78", "1-", "[x-z]", "X", false, "12X34X56X78\n");
        assert_cut_fields_with_regex("12x34y56z78", "3-", "[x-z]", "X", false, "56X78\n");

        // Fixed length delimiter.
        assert_cut_fields_with_regex("ab, cd, ef, gh", "1-", ", ", "!", false, "ab!cd!ef!gh\n");
        assert_cut_fields_with_regex("ab, cd, ef, gh", "2-3", ", ", "~", false, "cd~ef\n");
        assert_cut_fields_with_regex("a1234b5678c", "1-", r"\d{3}", "*", false, "a*4b*8c\n");
        assert_cut_fields_with_regex("a1234b5678c", "1,3", r"\d{3}", "*", false, "a*8c\n");

        // Variable length delimiter.
        assert_cut_fields_with_regex("a1b23c456d", "1-", r"\d+", "&", false, "a&b&c&d\n");
        assert_cut_fields_with_regex("a1b23c456d", "1,4", r"\d+", "&", false, "a&d\n");
        assert_cut_fields_with_regex("axbxycxyzdzyyxe", "1-", "[xyz]+", "^", false, "a^b^c^d^e\n");
        assert_cut_fields_with_regex("axbxycxyzdzyyxe", "1,5", "[xyz]+", "^", false, "a^e\n");
        assert_cut_fields_with_regex(
            "a0b-c1-d-2-e",
            "1-",
            r"(\d|-)+",
            "__",
            false,
            "a__b__c__d__e\n",
        );
        assert_cut_fields_with_regex(
            "a0b-c1-d-2-e",
            "3-",
            r"(\d|-)+",
            "|~|",
            false,
            "c|~|d|~|e\n",
        );
    }

    fn assert_cut_fields_with_regex(
        input: &str,
        ranges: &str,
        delimiter: &str,
        joiner: &str,
        suppress: bool,
        expected: &str,
    ) {
        let mut output = Vec::new();
        let ranges: Ranges = ranges.parse().unwrap();
        super::cut_fields_with_regex(
            &mut input.as_bytes(),
            &mut output,
            &Regex::new(delimiter).unwrap(),
            &joiner.to_string(),
            suppress,
            &ranges,
        )
        .unwrap();
        let actual = String::from_utf8(output).unwrap();
        assert_eq!(actual, expected);
    }
}
