# Rut
`rut` is a tool for selecting bytes, characters, or fields from files. It is heavily based on
`cut`, but with additional support for delimiting fields with a regular expression. Note that there
are multiple implementations of `cut` which differ in various ways. `rut` is primarily based on
the GNU Coreutils implementation.

Below is a table briefly describing `rut`'s features and how it compares to `cut`.
| Option | Description | [POSIX `cut`](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/cut.html) <sup>1</sup> | [GNU Coreutils `cut`](https://www.gnu.org/software/coreutils/cut) | `rut` |
|:--|:--|:--|:--|:--|
| `-b` | Select bytes. | ✔ | ✔ (also supports `--bytes`) | ✔ (also supports `--bytes`) |
| `-c` | Select characters. | ✔ | ⚠ (also supports `--characters`; behaves the same as `-b`) | ✔ (also supports `--characters`; requires UTF-8 input) |
| `-f` | Select fields (strings separated by a delimiter). | ✔ | ✔ (also supports `--fields`; treats each byte as a character, without regard for encoding) | ✔ (also supports `--fields`; requires UTF-8 input) |
| `-d` | Specify a single character delimiter when used with `-f`. | ✔ | ⚠ (also supports `--delimiter`; requires single byte character) | ✔ (also supports `--delimiter`; must be a UTF-8 character) |
| `-s` | Do not print lines without a delimiter. Normal behavior is to print the full line. | ✔ | ✔ (also supports `--only-delimited`) | ✔ (also supports `--only-delimited`) |
| `-n` | Do not split multi-byte characters when used with `-b`. | ✔ | ⚠ (no-op) | ⚠ (no-op) |
| `--output-delimiter` | Specify a string to delimit selected fields. Normal behavior is to use delimiter. | ❌ | ✔ | ✔ (also supports `-o`) |
| `--complement` | Select the complement of selected bytes/characters/fields. | ❌ | ✔ | ✔ |
| `-z` / `--zero-terminated` | Delimit "lines" with a zero byte rather than a newline | ❌ | ✔ | ✔ |
| `-r` / `--regex-delimiter` | Specific a regular expression as a delimiter when used with `-f`. | ❌ | ❌ | ✔ |

1. This column describes the POSIX definition of `cut` and not any particular implementation.

## Use
`rut` is intended to be a drop-in replacement for the GNU Coreutils implementation of `cut` in many
cases. In particular, you should be able to replace `cut` with `rut` for any *valid* `cut` command
with ASCII input. It is considered a bug if it does not.

`rut` also adds support for an additional options and UTF-8 encoded input. This has the following
consequences.
* Some `cut` commands which would fail due to invalid or unrecognized options will pass with `rut`.
* The output for commands using `-c` or `-f` will be different for non-ASCII input.

### Examples

Select bytes from a file:
```bash
$ rut -b1-5 tests/files/ascii.txt
abcde
a b c
a_b_c
a:b:c
```

Select from stdin:
```bash
$ cat tests/files/ascii.txt | rut -b1-5
abcde
a b c
a_b_c
a:b:c
```

Comparison of bytes versus characters:
```bash
$ rut -b1-4 tests/files/utf8.txt
abcd
αβ
abα
😀
$ rut -c1-4 tests/files/utf8.txt
abcd
αβγδ
abαβ
😀😁😂😃
```

Select fields:
```bash
$ rut -f1 -dd tests/files/ascii.txt
abc
a b c 
a_b_c_
$ rut -f2,4,6 -d'_' -s tests/files/ascii.txt
b_d_f
```

Select fields with regex delimiter:
```bash
$ rut -f 2-4,6-8 -r '[ _:]' -s -o# tests/files/ascii.txt
b#c#d#f#g#h
b#c#d#f#g#h
b#c#d#f#g#h
```

## Test and Build
`rut` is written in [Rust](https://www.rust-lang.org/). It has been tested with Rust 1.45.2 but may
work with earlier or later versions. The following instructions assume you have Rust installed.

To run the full test suite, run:
```bash
$ cargo test
```

To build `rut` in debug mode (faster compile time, slower executable), use:
```bash
$ cargo build
```

For release mode (slower compile times, faster executable), use:
```bash
$ cargo build --release
```

## Package for Release
Build scripts are provided to create a release package. Follow the instructions
below to build an archive (`.zip` or `.tar.gz`) in the `target` directory.

### Windows
```powershell
$ .\Create-Release.ps1
```

### Linux
```bash
$ ./create-release.sh
```
