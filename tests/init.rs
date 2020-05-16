use assert_cmd::Command;
use predicates::prelude::*;

fn cmd(sub: &str) -> Command {
  let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
  cmd.arg(sub);

  cmd
}

#[test]
fn help() {
  use predicate::str::contains;

  cmd("init")
    .args(&["--help"])
    .assert()
    .code(0)
    .stdout(contains("pidgit init"));

  cmd("init")
    .args(&["-h"])
    .assert()
    .code(0)
    .stdout(contains("pidgit init"));
}
