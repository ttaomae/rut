pub mod util;

#[test]
fn multiple_modes() {
    assert_invalid_options(&["-b1", "-b1"]);
    assert_invalid_options(&["-c1", "-c1"]);
    assert_invalid_options(&["-f1", "-f1"]);

    assert_invalid_options(&["-b1", "-c1"]);
    assert_invalid_options(&["-b1", "-f1"]);
    assert_invalid_options(&["-c1", "-f1"]);
    assert_invalid_options(&["-b1", "-c1", "-f1"]);
}

#[test]
fn missing_or_invalid_ranges() {
    assert_invalid_options(&["-b"]);
    assert_invalid_options(&["-c"]);
    assert_invalid_options(&["-f"]);

    assert_invalid_options(&["-b2-1"]);
    assert_invalid_options(&["-c0-3"]);
    assert_invalid_options(&["-fxyz"]);
}

#[test]
fn field_options_with_byte_mode() {
    assert_invalid_options(&["-b1", "-s"]);
    assert_invalid_options(&["-b1", "-d_"]);
    assert_invalid_options(&["-b1", "-r_"]);
    assert_invalid_options(&["-b1", "-o_"]);
}

#[test]
fn field_options_with_char_mode() {
    assert_invalid_options(&["-c1", "-s"]);
    assert_invalid_options(&["-c1", "-d_"]);
    assert_invalid_options(&["-c1", "-r_"]);
    assert_invalid_options(&["-c1", "-o_"]);
}

#[test]
fn repeated_options() {
    assert_invalid_options(&["-f1", "-d_", "-d-"]);
    assert_invalid_options(&["-f1", "-r_", "-r-"]);
    assert_invalid_options(&["-f1", "-o_", "-o-"]);
    assert_invalid_options(&[]);
}

#[test]
fn n_with_non_byte_mode() {
    assert_invalid_options(&["-c1", "-n"]);
    assert_invalid_options(&["-f1", "-n"]);
}

fn assert_invalid_options(options: &[&str]) {
    util::test_command()
        .options(options)
        .build()
        .assert()
        .failure();
}
