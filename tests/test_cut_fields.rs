pub mod util;

#[test]
fn ascii_char_delimiter() {
    util::test_command()
        .option("-f3,6,9")
        .option("-d ")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "abcdefghijklmnopqrstuvwxyz
c f i
a_b_c_d_e_f_g_h_i_j_k_l_m
a:b:c:d:e:f:g:h:i:j:k:l:m
",
        );

    util::test_command()
        .option("-f3,6,9")
        .option("-d_")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "abcdefghijklmnopqrstuvwxyz
a b c d e f g h i j k l m
c_f_i
a:b:c:d:e:f:g:h:i:j:k:l:m
",
        );

    util::test_command()
        .option("-f3,6,9")
        .option("-d:")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "abcdefghijklmnopqrstuvwxyz
a b c d e f g h i j k l m
a_b_c_d_e_f_g_h_i_j_k_l_m
c:f:i
",
        );
}

#[test]
fn utf8_char_delimiter() {
    util::test_command()
        .option("-f2")
        .option("-dg")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "hijklm
αβγδεζηθικλμν
abαβcdγδefεζ
😀😁😂😃😄😅😆😇😈
",
        );

    util::test_command()
        .option("-f2")
        .option("-dζ")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "abcdefghijklm
ηθικλμν

😀😁😂😃😄😅😆😇😈
",
        );

    util::test_command()
        .option("-f2")
        .option("-dd")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "efghijklm
αβγδεζηθικλμν
γδefεζ
😀😁😂😃😄😅😆😇😈
",
        );

    util::test_command()
        .option("-f2")
        .option("-d😄")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "abcdefghijklm
αβγδεζηθικλμν
abαβcdγδefεζ
😅😆😇😈
",
        );
}

#[test]
fn ascii_regex_delimiter() {
    util::test_command()
        .option("-f2,4")
        .option("-r[aeiou]")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "bcd\tjklmn
 b c d \t j k l m
_b_c_d_\t_j_k_l_m
:b:c:d:\t:j:k:l:m
",
        );
}

#[test]
fn utf8_regex_delimiter() {
    util::test_command()
        .option("-f2")
        .option("-r[dδ😄]")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "efghijklm
εζηθικλμν
γ
😅😆😇😈
",
        );
}

#[test]
fn ranges_complement() {
    util::test_command()
        .option("-f3-6,9-12")
        .option("--complement")
        .option("--only-delimited")
        .option("--regex-delimiter=[ _:]")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "a\tb\tg\th\tm
a\tb\tg\th\tm
a\tb\tg\th\tm
",
        );
}

#[test]
fn empty_ranges() {
    util::test_command()
        .option("-f1-")
        .option("--complement")
        .option("--regex-delimiter=.")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("\n\n\n\n");
}

#[test]
fn zero_terminated_ascii() {
    util::test_command()
        .option("-f2-3")
        .option("-d_")
        .option("-z")
        .option("-s")
        .file("tests/files/ascii-zero.txt")
        .build()
        .assert()
        .code(0)
        .stdout("b_c\0");
}

#[test]
fn zero_terminated_utf8() {
    util::test_command()
        .option("-f1")
        .option("-dβ")
        .option("-z")
        .option("-s")
        .file("tests/files/utf8-zero.txt")
        .build()
        .assert()
        .code(0)
        .stdout("α\0abα\0");
}

#[test]
fn output_delimiter_and_suppress() {
    util::test_command()
        .option("-f5,10")
        .option("-d_")
        .option("-s")
        .option("-o-")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("e-j\n");

    util::test_command()
        .option("-f5,10")
        .option("-d_")
        .option("-s")
        .option("-o___")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("e___j\n");

    util::test_command()
        .option("-f2,4,6")
        .option(r"-r_\w_")
        .option("-s")
        .option("-o#")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("c#g#k\n");

    util::test_command()
        .option("-f2,4,6")
        .option(r"-r\s\w\s")
        .option("-s")
        .option("-o***")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("c***g***k\n");
}

#[test]
fn from_stdin() {
    util::test_command()
        .option("-f1,3")
        .build()
        .write_stdin("a\tb\tc\nd\te\tf\ng\th\ti")
        .assert()
        .code(0)
        .stdout("a\tc\nd\tf\ng\ti\n");
}

#[test]
fn file_and_stdin() {
    util::test_command()
        .option("-f2,4")
        .option("-r[_:]")
        .option("-s")
        .file("tests/files/ascii.txt")
        .file("-")
        .build()
        .write_stdin("z y x w v\tz_y_x_w_v\nz:y:x:w:v")
        .assert()
        .code(0)
        .stdout(
            "b\td
b\td
y\tw
y\tw
",
        );
}

#[test]
fn multiple_files() {
    util::test_command()
        .option("-f2-3")
        .option("-r[aeiou]")
        .option("-s")
        .file("tests/files/ascii.txt")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout(
            "bcd\tfgh
 b c d \t f g h \n_b_c_d_\t_f_g_h_
:b:c:d:\t:f:g:h:
bcd\tfgh
bαβcdγδ\tfεζ
",
        );
}

#[test]
fn missing_file() {
    util::test_command()
        .option("-f3,6,9,12")
        .option("-d_")
        .option("-s")
        .file("tests/files/ascii.txt")
        .file("tests/files/unknown.txt")
        .build()
        .assert()
        .code(1)
        .stdout("c_f_i_l\n");
}
