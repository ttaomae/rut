pub mod util;

#[test]
fn all_ascii_chars() {
    util::test_command()
        .option("-c1-")
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
fn all_utf8_chars() {
    util::test_command()
        .option("-c1-")
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
        .option("-c2,4,8,16-21")
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
fn some_utf8_chars() {
    util::test_command()
        .option("-c1-4")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout("abcd
αβγδ
abαβ
😀😁😂😃
");
}

#[test]
fn zero_terminated_ascii() {
    util::test_command()
        .option("-c1,3,5")
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
        .option("-c1,3,5")
        .option("-z")
        .file("tests/files/utf8-zero.txt")
        .build()
        .assert()
        .code(0)
        .stdout("ace\0αγε\0aαc\0😀😂😄\0");
}

#[test]
fn from_stdin() {
    // No file specified.
    util::test_command()
        .option("-c1")
        .build()
        .write_stdin("abc\na b\na_b\na:b")
        .assert()
        .code(0)
        .stdout("a\na\na\na\n");

    // "-" file.
    util::test_command()
        .option("-c3")
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
        .option("-c5,10,15,20,25")
        .file("tests/files/ascii.txt")
        .file("tests/files/utf8.txt")
        .build()
        .assert()
        .code(0)
        .stdout("ejoty
c h m
c_h_m
c:h:m
ej
εκ
cf
😄
");
}

#[test]
fn file_and_stdin() {
    util::test_command()
        .option("-c8-")
        .file("tests/files/utf8.txt")
        .file("-")
        .build()
        .write_stdin("abcdefgh
ijklmnop
qrstuvwx")
        .assert()
        .code(0)
        .stdout("hijklm
θικλμν
δefεζ
😇😈
h
p
x
");
}

#[test]
fn missing_file() {
    util::test_command()
        .option("-c1,3,5")
        .file("tests/files/utf8.txt")
        .file("tests/files/unknown.txt")
        .build()
        .assert()
        .code(1)
        .stdout("ace
αγε
aαc
😀😂😄
");
}

#[test]
fn non_utf8() {
    util::test_command()
        .option("-c1")
        .build()
        .write_stdin(vec![255, 254, 253, 252, 251])
        .assert()
        .code(1);
}
