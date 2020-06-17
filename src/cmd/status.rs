use clap::{App, ArgMatches};
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::cmd::Context;
use crate::prelude::*;

#[derive(Debug)]
struct Status;

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

    let mut untracked = BTreeSet::new();
    let workspace = repo.workspace();

    self.scan_workspace(workspace.root(), repo, &mut untracked)?;

    for spec in untracked {
      ctx.println(format!("?? {}", spec));
    }

    Ok(())
  }
}

impl Status {
  fn scan_workspace(
    &self,
    base: &PathBuf,
    repo: &Repository,
    untracked: &mut BTreeSet<String>,
  ) -> Result<()> {
    let ws = repo.workspace();
    for (path, stat) in ws.list_dir(base)? {
      let is_dir = stat.is_dir();

      if repo.index()?.is_path_tracked(&path) {
        if is_dir {
          self.scan_workspace(&ws.canonicalize(&path), repo, untracked)?;
        }
      } else {
        let suffix = if is_dir {
          std::path::MAIN_SEPARATOR.to_string()
        } else {
          "".to_string()
        };

        untracked.insert(format!("{}{}", path.display(), suffix));
      }
    }

    Ok(())
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
    assert!(stdout.contains("?? dir/\n"));
    assert!(stdout.contains("?? file.txt"));
  }
}
