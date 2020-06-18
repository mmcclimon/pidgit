use clap::{App, ArgMatches};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::Metadata;
use std::path::PathBuf;

use crate::cmd::Context;
use crate::index::Index;
use crate::prelude::*;

#[derive(Debug)]
struct Status;

struct StatusHelper<'c> {
  repo:  &'c Repository,
  index: &'c Index,
}

pub fn new() -> Box<dyn Command> {
  Box::new(Status {})
}

impl Command for Status {
  fn app(&self) -> App<'static, 'static> {
    // this doesn't have all the smarts git does, for now
    App::new("status").about("show the working tree status")
  }

  fn run(&self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;

    let helper = StatusHelper {
      repo,
      index: &repo.index()?,
    };

    let mut untracked = BTreeSet::new();
    let workspace = repo.workspace();

    helper.scan_workspace(workspace.root(), &mut untracked)?;

    for spec in untracked {
      ctx.println(format!("?? {}", PathBuf::from(spec).display()));
    }

    Ok(())
  }
}

impl StatusHelper<'_> {
  fn scan_workspace(
    &self,
    base: &PathBuf,
    untracked: &mut BTreeSet<OsString>,
  ) -> Result<()> {
    let ws = self.repo.workspace();
    for (path_str, stat) in ws.list_dir(base)? {
      let is_dir = stat.is_dir();

      if self.index.is_tracked(&path_str) {
        if is_dir {
          self.scan_workspace(&ws.canonicalize(&path_str), untracked)?;
        }
      } else if self.is_trackable(&path_str, &stat) {
        let suffix = if is_dir {
          std::path::MAIN_SEPARATOR.to_string()
        } else {
          "".to_string()
        };

        let mut name = path_str.clone();
        name.push(suffix);

        untracked.insert(name);
      }
    }

    Ok(())
  }

  // a path is trackable iff it contains a file somewhere inside it.
  fn is_trackable(&self, path: &OsString, stat: &Metadata) -> bool {
    if stat.is_file() {
      return !self.index.is_tracked(&path);
    }

    if !stat.is_dir() {
      return false;
    }

    let ws = self.repo.workspace();

    ws.list_dir(&path.into())
      .expect(&format!("could not list dir {:?}", path))
      .iter()
      .any(|(path, stat)| self.is_trackable(path, stat))
  }
}

#[cfg(test)]
mod tests {
  use crate::test_prelude::*;

  #[test]
  fn only_untracked_files() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "file content");
    tr.write_file("other.txt", "more content");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert!(stdout.contains("?? file.txt"));
    assert!(stdout.contains("?? other.txt"));
  }

  #[test]
  fn untracked_and_others() {
    let tr = new_empty_repo();
    tr.write_file("committed.txt", "to be committed");

    #[rustfmt::skip]
    tr.run_pidgit(vec!["add", "committed.txt"]).expect("bad add");
    tr.commit("a commit message").expect("could not commit");

    tr.write_file("file.txt", "uncommitted");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert!(stdout.contains("?? file.txt"));
    assert!(!stdout.contains("?? committed.txt"));
  }

  #[test]
  fn untracked_dirs() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "top-level file");
    tr.write_file("dir/nested.txt", "nested file");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_eq!(stdout, "?? dir/\n?? file.txt\n");
  }

  #[test]
  fn untracked_dirs_nested() {
    let tr = new_empty_repo();
    tr.write_file("a/b/inner.txt", "nested file");

    tr.run_pidgit(vec!["add", "."]).expect("bad add");
    tr.commit("a commit message").expect("could not commit");

    tr.write_file("a/outer.txt", "outer untracked file");
    tr.write_file("a/b/c/nested.txt", "more deeply nested file");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_eq!(stdout, "?? a/b/c/\n?? a/outer.txt\n");
  }

  #[test]
  fn no_empty_untracked_dirs() {
    let tr = new_empty_repo();
    tr.mkdir("outer");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_eq!(stdout, "");

    tr.write_file("outer/inner/file.txt", "a file");
    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_eq!(stdout, "?? outer/\n");
  }
}
