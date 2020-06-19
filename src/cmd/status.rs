use clap::{App, ArgMatches};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::fs::Metadata;
use std::path::PathBuf;

use crate::cmd::Context;
use crate::index::Index;
use crate::prelude::*;

#[derive(Debug)]
struct Status;

enum ChangeType {
  Modified,
  Deleted,
}

struct StatusHelper<'c> {
  repo:      &'c Repository,
  index:     &'c mut Index,
  untracked: BTreeSet<OsString>,
  changed:   BTreeMap<OsString, ChangeType>,
  stats:     HashMap<OsString, Metadata>,
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

    let mut helper = StatusHelper {
      repo,
      index: &mut repo.index()?,
      untracked: BTreeSet::new(),
      changed: BTreeMap::new(),
      stats: HashMap::new(),
    };

    let workspace = repo.workspace();

    helper.scan_workspace(workspace.root())?;
    helper.detect_changes();

    for (file, status) in helper.changed {
      ctx.println(format!(
        " {} {}",
        status.short_desc(),
        PathBuf::from(file).display()
      ));
    }

    for spec in helper.untracked {
      ctx.println(format!("?? {}", PathBuf::from(spec).display()));
    }

    // update the index, in case any of the stats have changed
    helper.index.write()?;

    Ok(())
  }
}

impl ChangeType {
  fn short_desc(&self) -> &'static str {
    match self {
      Self::Modified => "M",
      Self::Deleted => "D",
    }
  }
}

impl StatusHelper<'_> {
  fn scan_workspace(&mut self, base: &PathBuf) -> Result<()> {
    let ws = self.repo.workspace();
    for (path_str, stat) in ws.list_dir(base)? {
      let is_dir = stat.is_dir();

      if self.index.is_tracked(&path_str) {
        if is_dir {
          self.scan_workspace(&ws.canonicalize(&path_str))?;
        } else if stat.is_file() {
          self.stats.insert(path_str.clone(), stat.clone());
        }
      } else if self.is_trackable(&path_str, &stat) {
        let suffix = if is_dir {
          std::path::MAIN_SEPARATOR.to_string()
        } else {
          "".to_string()
        };

        let mut name = path_str.clone();
        name.push(suffix);

        self.untracked.insert(name);
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

  fn detect_changes(&mut self) {
    // For every file in the index, if our stat is different than it, it's
    // changed.
    for entry in self.index.entries_mut() {
      let path = &entry.name;
      let stat = self.stats.get(path);

      if stat.is_none() {
        self.changed.insert(path.clone(), ChangeType::Deleted);
        continue;
      }

      let stat = stat.unwrap();

      if !entry.matches_stat(stat) {
        self.changed.insert(path.clone(), ChangeType::Modified);
        continue;
      }

      if entry.matches_time(stat) {
        continue;
      }

      // Check the content
      let sha = util::compute_sha_for_path(
        &self.repo.workspace().canonicalize(path),
        Some(stat),
      )
      .expect("could not calculate sha")
      .hexdigest();

      if sha == entry.sha {
        // if we've gotten here, we know the index stat time is stale
        entry.update_meta(stat);
        continue;
      } else {
        self.changed.insert(path.clone(), ChangeType::Modified);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test_prelude::*;

  fn assert_status(stdout: String, want: &str) {
    let expect = want.to_string() + "\n";
    assert_eq!(stdout, expect);
  }

  #[test]
  fn only_untracked_files() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "file content");
    tr.write_file("other.txt", "more content");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, "?? file.txt\n?? other.txt");
  }

  #[test]
  fn untracked_and_others() {
    let tr = new_empty_repo();
    tr.write_file("committed.txt", "to be committed");
    tr.commit_all();

    tr.write_file("file.txt", "uncommitted");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, "?? file.txt");
  }

  #[test]
  fn untracked_dirs() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "top-level file");
    tr.write_file("dir/nested.txt", "nested file");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, "?? dir/\n?? file.txt");
  }

  #[test]
  fn untracked_dirs_nested() {
    let tr = new_empty_repo();
    tr.write_file("a/b/inner.txt", "nested file");
    tr.commit_all();

    tr.write_file("a/outer.txt", "outer untracked file");
    tr.write_file("a/b/c/nested.txt", "more deeply nested file");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, "?? a/b/c/\n?? a/outer.txt");
  }

  #[test]
  fn no_empty_untracked_dirs() {
    let tr = new_empty_repo();
    tr.mkdir("outer");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_eq!(stdout, "");

    tr.write_file("outer/inner/file.txt", "a file");
    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, "?? outer/");
  }

  #[test]
  fn simple_modification() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "original content\n");
    tr.commit_all();

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_eq!(stdout, "");

    tr.write_file("file.txt", "original_content\nplus another line\n");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, " M file.txt");
  }

  #[test]
  fn change_mode() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "original content\n");
    tr.commit_all();

    tr.chmod("file.txt", 0o755);

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, " M file.txt");
  }

  #[test]
  fn modify_same_size() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "2 cats\n");
    tr.commit_all();

    tr.write_file("file.txt", "9 cats\n");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, " M file.txt");
  }

  #[test]
  fn deleted_files() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "meh\n");
    tr.commit_all();

    tr.rm_file("file.txt");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert_status(stdout, " D file.txt");
  }
}
