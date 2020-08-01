pub mod util;

#[test]
fn all_ascii_bytes() {
    util::test_command()
        .option("-b1-")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("abcdefghijklmnopqrstuvwxyz
a b c d e f g h i j k l m
a_b_c_d_e_f_g_h_i_j_k_l_m
a:b:c:d:e:f:g:h:i:j:k:l:m
");
}

#[test]
fn all_utf8_bytes() {
    util::test_command()
        .option("-b1-")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout("abcdefghijklm
αβγδεζηθικλμν
abαβcdγδefεζ
😀😁😂😃😄😅😆😇😈
");
}

#[test]
fn some_ascii_bytes() {
    util::test_command()
        .option("-b2,4,8,16-21")
        .file("tests/files/ascii.txt")
        .build()
        .assert()
        .code(0)
        .stdout("bdhpqrstu
    i j k
____i_j_k
::::i:j:k
");
}

#[test]
fn some_utf8_bytes() {
    util::test_command()
        .option("-b1-4")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout("abcd
αβ
abα
😀
");
}

#[test]
fn all_binary_bytes() {
    util::test_command()
        .option("-b1-")
        .file("tests/files/bytes.bin")
        .build()
        .assert()
        .code(0)
        .stdout(&[1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 15, 10, 21, 22, 23, 24, 25, 10, 31, 32, 33, 34, 35, 10][..]);
}

#[test]
fn some_binary_bytes() {
    util::test_command()
        .option("-b1,3,5")
        .file("tests/files/bytes.bin")
        .build()
        .assert()
        .code(0)
        .stdout(&[1,3,5,10,11,13,15,10,21,23,25,10,31,33,35,10][..]);
}

#[test]
fn zero_terminated_ascii() {
    util::test_command()
        .option("-b1,3,5")
        .option("-z")
        .file("tests/files/ascii-zero.txt")
        .build()
        .assert()
        .code(0)
        .stdout("ace\0abc\0abc\0abc\0");
}

#[test]
fn zero_terminated_utf8() {
    util::test_command()
        .option("-b1-4")
        .option("-z")
        .file("tests/files/utf8-zero.txt")
        .build()
        .assert()
        .code(0)
        .stdout("abcd\0αβ\0abα\0😀\0");
}

#[test]
fn zero_terminated_binary() {
    util::test_command()
        .option("-b2,4")
        .option("-z")
        .file("tests/files/bytes-zero.bin")
        .build()
        .assert()
        .code(0)
        .stdout(&[2,4,0,12,14,0,22,24,0,32,34,0][..]);
}


#[test]
fn from_stdin() {
    // No file specified.
    util::test_command()
        .option("-b1")
        .build()
        .write_stdin("abc\na b\na_b\na:b")
        .assert()
        .code(0)
        .stdout("a\na\na\na\n");

    // "-" file.
    util::test_command()
        .option("-b3")
        .file("-")
        .build()
        .write_stdin("abc\na b\na_b\na:b")
        .assert()
        .code(0)
        .stdout("c\nb\nb\nb\n");
}

#[test]
fn multiple_files() {
    util::test_command()
        .option("-b5-8")
        .file("tests/files/ascii.txt")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout("efgh
c d \nc_d_
c:d:
efgh
γδ
βcd
😁
");
}

#[test]
fn file_and_stdin() {
    util::test_command()
        .option("-b1-3")
        .file("-")
        .file("tests/files/ascii.txt")
        .build()
        .write_stdin("abcdef
ghijkl
mnopqr")
        .assert()
        .code(0)
        .stdout("abc
ghi
mno
abc
a b
a_b
a:b
");
}

#[test]
fn missing_file() {
    util::test_command()
        .option("-b2-3,5,7,11")
        .file("tests/files/ascii.txt")
        .file("tests/files/unknown.txt")
        .build()
        .assert()
        .code(1)
        .stdout("bcegk
 bcdf
_bcdf
:bcdf
");
}
