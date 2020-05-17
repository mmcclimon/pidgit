use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serial_test::serial;

use predicate::str::contains;

// eventually, all this stuff will be listed into some common module

// make a tempdir and cd to it
fn cd_temp() -> TempDir {
  let dir = TempDir::new().unwrap();
  std::env::set_current_dir(&dir.path()).unwrap();
  dir
}

fn cmd(sub: &str) -> Command {
  let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
  cmd.arg(sub);
  cmd
}

#[test]
fn init_help() {
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

#[test]
#[serial]
fn init_dot_pidgit_exists() {
  let dir = cd_temp();
  let pidgit_path = dir.child(".pidgit");
  pidgit_path.create_dir_all().unwrap();
  pidgit_path.assert(predicate::path::is_dir());

  cmd("init")
    .assert()
    .code(0)
    .stdout(contains("nothing to do"));
}

#[test]
#[serial]
fn init_create_dir() {
  use predicate::path::is_dir;

  let dir = cd_temp();

  cmd("init")
    .assert()
    .code(0)
    .stdout(contains("initialized empty pidgit repository"));

  let ppath = dir.child(".pidgit");

  ppath.child("HEAD").assert(contains("refs/heads/master"));
  ppath
    .child("config")
    .assert(contains("repositoryformatversion = 0"));

  ppath.child("objects").assert(is_dir());
  ppath.child("refs").child("heads").assert(is_dir());
  ppath.child("refs").child("tags").assert(is_dir());
  ppath.child("refs").child("remotes").assert(is_dir());
}
