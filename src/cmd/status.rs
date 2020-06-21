use clap::{App, Arg, ArgMatches};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::fs::Metadata;
use std::path::PathBuf;

use crate::cmd::Context;
use crate::index::Index;
use crate::object::{PathEntry, TreeItem};
use crate::prelude::*;

#[derive(Debug)]
struct Status;

#[derive(Debug, Eq, PartialEq, Hash)]
enum ChangeType {
  Modified,
  Deleted,
  Added,
  Untracked,
}

#[derive(Debug)]
struct StatusHelper<'c> {
  repo:           &'c Repository,
  index:          &'c mut Index,
  stats:          HashMap<OsString, Metadata>,
  untracked:      BTreeMap<OsString, ChangeType>,
  index_diff:     BTreeMap<OsString, ChangeType>,
  workspace_diff: BTreeMap<OsString, ChangeType>,
  head_diff:      HashMap<OsString, PathEntry>,
}

pub fn new() -> Box<dyn Command> {
  Box::new(Status {})
}

impl Command for Status {
  fn app(&self) -> App<'static, 'static> {
    // this doesn't have all the smarts git does, for now
    App::new("status")
      .about("show the working tree status")
      .arg(
        Arg::with_name("short")
          .long("short")
          .short("s")
          .help("show status concisely"),
      )
      .arg(
        Arg::with_name("porcelain")
          .long("porcelain")
          .help("machine-readable output"),
      )
  }

  fn run(&self, matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;

    // we accumulate state in the helper, then print it all out.
    let mut helper = StatusHelper {
      repo,
      index: &mut repo.index()?,
      untracked: BTreeMap::new(),
      stats: HashMap::new(),
      head_diff: HashMap::new(),
      index_diff: BTreeMap::new(),
      workspace_diff: BTreeMap::new(),
    };

    let workspace = repo.workspace();

    helper.scan_workspace(workspace.root())?;
    helper.load_head()?;
    helper.detect_changes();

    // print!
    if matches.is_present("short") {
      helper.print_short(ctx, true);
    } else if matches.is_present("porcelain") {
      helper.print_short(ctx, false);
    } else {
      helper.print_full(ctx);
    }
    // update the index, in case any of the stats have changed
    helper.index.write()?;

    Ok(())
  }
}

impl ChangeType {
  fn display(&self) -> &'static str {
    match self {
      Self::Modified => "M",
      Self::Deleted => "D",
      Self::Added => "A",
      Self::Untracked => "?",
    }
  }

  fn long_display(&self) -> &'static str {
    match self {
      Self::Modified => "modified",
      Self::Deleted => "deleted",
      Self::Added => "new file",
      Self::Untracked => "untracked file",
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

        self.untracked.insert(name, ChangeType::Untracked);
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

  fn load_head(&mut self) -> Result<()> {
    let head = self.repo.head();
    if head.is_none() {
      return Ok(());
    }

    let tree = head.unwrap().tree().to_string();
    self.read_tree(&tree, &PathBuf::from(""))?;

    Ok(())
  }

  fn read_tree(&mut self, sha: &str, prefix: &PathBuf) -> Result<()> {
    let tree = self.repo.resolve_object(sha)?.as_tree()?;

    for (path, entry) in tree.entries() {
      if let TreeItem::Entry(e) = entry {
        let fullpath = prefix.join(path);
        if e.is_tree() {
          self.read_tree(e.sha(), &fullpath)?;
        } else {
          self.head_diff.insert(fullpath.into(), e.clone());
        }
      }
    }

    Ok(())
  }

  fn detect_changes(&mut self) {
    self.check_index();
    self.check_head();
  }

  fn check_index(&mut self) {
    // Check the working tree: for every file in the index, if our stat is
    // different than it, it's changed.
    for entry in self.index.entries_mut() {
      let path = &entry.name;
      let stat = self.stats.get(path);

      if stat.is_none() {
        self
          .workspace_diff
          .insert(path.clone(), ChangeType::Deleted);
        continue;
      }

      let stat = stat.unwrap();

      if !entry.matches_stat(stat) {
        self
          .workspace_diff
          .insert(path.clone(), ChangeType::Modified);
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
        self
          .workspace_diff
          .insert(path.clone(), ChangeType::Modified);
      }
    }
  }

  fn check_head(&mut self) {
    // now, check against the head
    for entry in self.index.entries() {
      let path = &entry.name;

      if !self.head_diff.contains_key(path) {
        self.index_diff.insert(path.clone(), ChangeType::Added);
        continue;
      }

      let have = self.head_diff.get(path).unwrap();

      if have.mode() != entry.mode() || have.sha() != entry.sha {
        self.index_diff.insert(path.clone(), ChangeType::Modified);
        continue;
      }
    }

    // check for files that exist in HEAD but aren't in the index
    for key in self.head_diff.keys() {
      if !self.index.is_tracked_file(key) {
        self.index_diff.insert(key.clone(), ChangeType::Deleted);
      }
    }
  }

  fn status_for(&self, path: &OsString, use_color: bool) -> String {
    let left = match self.index_diff.get(path) {
      Some(ct) => ct.display(),
      None => " ",
    };

    let right = match self.workspace_diff.get(path) {
      Some(ct) => ct.display(),
      _ => " ",
    };

    if use_color {
      format!(
        "{}{}",
        util::colored(left, Color::Green),
        util::colored(right, Color::Red)
      )
    } else {
      format!("{}{}", left, right)
    }
  }

  fn print_short(&self, ctx: &Context, use_color: bool) {
    use std::iter::FromIterator;

    let paths = BTreeSet::from_iter(
      self.index_diff.keys().chain(self.workspace_diff.keys()),
    );

    for path in paths {
      ctx.println(format!(
        "{} {}",
        self.status_for(&path, use_color),
        PathBuf::from(path).display()
      ));
    }

    for spec in self.untracked.keys() {
      ctx.println(format!("?? {}", PathBuf::from(spec).display()));
    }
  }

  fn print_full(&self, ctx: &Context) {
    self.print_changes(
      ctx,
      "Changes to be committed",
      &self.index_diff,
      Color::Green,
    );
    self.print_changes(
      ctx,
      "Changes not staged for commit",
      &self.workspace_diff,
      Color::Red,
    );
    self.print_changes(ctx, "Untracked files", &self.untracked, Color::Red);
    self.print_commit_status(ctx);
  }

  fn print_changes(
    &self,
    ctx: &Context,
    prefix: &str,
    changeset: &BTreeMap<OsString, ChangeType>,
    color: Color,
  ) {
    if changeset.len() == 0 {
      return;
    }

    ctx.println(format!("{}:", prefix));

    for (path, kind) in changeset {
      let status = if let ChangeType::Untracked = kind {
        "".to_string()
      } else {
        format!("{:<12}", kind.long_display().to_string() + ":")
      };

      ctx.println_color(
        format!("\t{}{}", status, PathBuf::from(path).display()),
        color,
      );
    }

    ctx.println("".to_string());
  }

  fn print_commit_status(&self, ctx: &Context) {
    if self.index_diff.len() > 0 {
      return;
    }

    if self.workspace_diff.len() > 0 {
      ctx.println("no changes added to commit".into())
    } else if self.untracked.len() > 0 {
      ctx.println("nothing added to commit but untracked files present".into())
    } else {
      ctx.println("nothing to commit, working tree clean".into())
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

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "?? file.txt\n?? other.txt");
  }

  #[test]
  fn untracked_and_others() {
    let tr = new_empty_repo();
    tr.write_file("committed.txt", "to be committed");
    tr.commit_all();

    tr.write_file("file.txt", "uncommitted");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "?? file.txt");
  }

  #[test]
  fn untracked_dirs() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "top-level file");
    tr.write_file("dir/nested.txt", "nested file");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "?? dir/\n?? file.txt");
  }

  #[test]
  fn untracked_dirs_nested() {
    let tr = new_empty_repo();
    tr.write_file("a/b/inner.txt", "nested file");
    tr.commit_all();

    tr.write_file("a/outer.txt", "outer untracked file");
    tr.write_file("a/b/c/nested.txt", "more deeply nested file");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "?? a/b/c/\n?? a/outer.txt");
  }

  #[test]
  fn no_empty_untracked_dirs() {
    let tr = new_empty_repo();
    tr.mkdir("outer");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_eq!(stdout, "");

    tr.write_file("outer/inner/file.txt", "a file");
    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "?? outer/");
  }

  #[test]
  fn simple_modification() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "original content\n");
    tr.commit_all();

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_eq!(stdout, "");

    tr.write_file("file.txt", "original_content\nplus another line\n");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, " M file.txt");
  }

  #[test]
  fn change_mode() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "original content\n");
    tr.commit_all();

    tr.chmod("file.txt", 0o755);

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, " M file.txt");
  }

  #[test]
  fn modify_same_size() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "2 cats\n");
    tr.commit_all();

    tr.write_file("file.txt", "9 cats\n");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, " M file.txt");
  }

  #[test]
  fn deleted_files() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "meh\n");
    tr.commit_all();

    tr.rm_file("file.txt");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, " D file.txt");
  }

  // tests below deal with diffing HEAD and the index, so we'll generate a
  // not-empty repo for testing
  fn new_with_commit() -> TestRepo {
    let tr = new_empty_repo();
    tr.write_file("1.txt", "one\n");
    tr.write_file("a/2.txt", "two\n");
    tr.write_file("a/b/3.txt", "three\n");
    tr.commit_all();
    tr
  }

  #[test]
  fn added_file_in_tracked_dir() {
    let tr = new_with_commit();

    tr.write_file("a/4.txt", "four\n");
    tr.run_pidgit(vec!["add", "."]).unwrap();

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "A  a/4.txt");
  }

  #[test]
  fn added_file_in_untracked_dir() {
    let tr = new_with_commit();

    tr.write_file("d/e/5.txt", "five\n");
    tr.run_pidgit(vec!["add", "."]).unwrap();

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "A  d/e/5.txt");
  }

  #[test]
  fn modified_mode() {
    let tr = new_with_commit();

    tr.chmod("1.txt", 0o755);
    tr.run_pidgit(vec!["add", "."]).unwrap();

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "M  1.txt");
  }

  #[test]
  fn modified_content() {
    let tr = new_with_commit();

    tr.write_file("a/b/3.txt", "tre\n");
    tr.run_pidgit(vec!["add", "."]).unwrap();

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "M  a/b/3.txt");
  }

  #[test]
  fn deleted_file_from_index() {
    let tr = new_with_commit();

    tr.rm_file("a/b/3.txt");
    tr.rm_file(".pidgit/index");
    tr.run_pidgit(vec!["add", "."]).expect("bad add!");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "D  a/b/3.txt");
  }

  #[test]
  fn deleted_dir_from_index() {
    let tr = new_with_commit();

    tr.rm_rf("a");
    tr.rm_file(".pidgit/index");
    tr.run_pidgit(vec!["add", "."]).expect("bad add!");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "D  a/2.txt\nD  a/b/3.txt");
  }
}
