// prelude for easy testing
pub use super::errors::{PidgitError, Result};
pub use crate as pidgit;
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

impl TestRepo {
  pub fn run_pidgit(&self, args: Vec<&str>) -> Result<String> {
    let app = pidgit::new();
    let mut stdout = Cursor::new(vec![]);

    let full_args = std::iter::once("pidgit").chain(args);
    let matches = app.clap_app().get_matches_from_safe(full_args)?;

    let repo = Some(&self.repo);

    app.dispatch(&matches, repo, &mut stdout, self.dir.path().to_path_buf())?;
    Ok(String::from_utf8(stdout.into_inner())?)
  }

  pub fn write_file(&self, filename: &str, content: &str) {
    self
      .dir
      .child(filename)
      .write_str(content)
      .expect(&format!("could not write file at {}", filename));
  }

  pub fn mkdir(&self, path: &str) {
    let child = self.dir.child(path);
    std::fs::create_dir_all(&child.path()).unwrap();
    assert!(child.path().is_dir())
  }

  pub fn rm_file(&self, path: &str) {
    let child = self.dir.child(path);
    std::fs::remove_file(&child.path()).expect("could not remove file");
    assert!(!child.path().exists())
  }

  pub fn rm_rf(&self, path: &str) {
    let child = self.dir.child(path);
    std::fs::remove_dir_all(&child.path()).expect("could not remove dir");
    assert!(!child.path().exists())
  }

  pub fn chmod(&self, pathstr: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let child = self.dir.child(pathstr);
    let path = child.path();
    let mut perms = path.metadata().expect("bad stat").permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).expect("could not chmod");
  }

  #[rustfmt::skip]
  pub fn commit_all(&self) {
    self.run_pidgit(vec!["add", "."]).expect("bad add");
    self.commit("auto commit message").expect("could not commit");
  }

  pub fn commit(&self, message: &str) -> Result<()> {
    use crate::object::Person;
    use chrono::Local;

    let now = Local::now();
    let fixed = now.with_timezone(now.offset());

    let who = Person {
      name:  "Pidgit".to_string(),
      email: "pidgit@example.com".to_string(),
      date:  fixed,
    };

    self.repo.commit(message, who.clone(), who).map(|_| ())
  }
}
