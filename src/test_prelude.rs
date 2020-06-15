// prelude for easy testing
pub use super::errors::{PidgitError, Result};
pub use assert_fs::prelude::*;
pub use assert_fs::TempDir;
pub use predicates::prelude::*;
pub use serial_test::serial;

use super::repo::Repository;
use std::io::Cursor;

// this is just so that the tempdir won't be dropped before the repo is
pub struct TestRepo {
  #[allow(unused)]
  dir:      TempDir,
  pub repo: Repository,
}

pub fn tempdir() -> TempDir {
  let d = TempDir::new().expect("couldn't make tempdir");
  assert!(d.path().is_dir());
  d
}

pub fn new_empty_repo() -> TestRepo {
  let dir = tempdir();

  let path = dir.path().canonicalize().unwrap();
  let repo = Repository::create_empty(&path).expect("could not init test repo");

  TestRepo { dir, repo }
}

pub fn run_pidgit(args: Vec<&str>, repo: Option<&Repository>) -> Result<String> {
  let app = super::new();
  let mut stdout = Cursor::new(vec![]);

  let full_args = std::iter::once("pidgit").chain(args);
  let matches = app.clap_app().get_matches_from_safe(full_args)?;

  app.dispatch(&matches, repo, &mut stdout)?;
  Ok(String::from_utf8(stdout.into_inner())?)
}

impl TestRepo {
  pub fn run_pidgit(&self, args: Vec<&str>) -> Result<String> {
    let app = super::new();
    let mut stdout = Cursor::new(vec![]);

    let full_args = std::iter::once("pidgit").chain(args);
    let matches = app.clap_app().get_matches_from_safe(full_args)?;

    let repo = Some(&self.repo);

    app.dispatch(&matches, repo, &mut stdout)?;
    Ok(String::from_utf8(stdout.into_inner())?)
  }
}
