mod support;

use std::process::Command;

#[test]
fn cli_prints_fixture_output() {
    let expected = support::read_fixture("hello-output.txt").expect("fixture should be readable");

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--print-hello")
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    assert_eq!(stdout, expected);
}
