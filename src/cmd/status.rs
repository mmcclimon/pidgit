use clap::{App, Arg, ArgMatches};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::PathBuf;

use crate::prelude::*;
use crate::repo::{ChangeType, Status};

struct StatusCmd {
  status: Status,
}

pub fn command() -> Command {
  (app, run)
}

pub fn app() -> ClapApp {
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

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;

  let status = repo.status()?;

  let cmd = StatusCmd { status };

  // print!
  if matches.is_present("short") {
    cmd.print_short(ctx, true);
  } else if matches.is_present("porcelain") {
    cmd.print_short(ctx, false);
  } else {
    cmd.print_full(ctx);
  }

  Ok(())
}

impl StatusCmd {
  fn status_for(&self, path: &OsString, use_color: bool) -> String {
    let left = match self.status.index_diff().get(path) {
      Some(ct) => ct.display(),
      None => " ",
    };

    let right = match self.status.workspace_diff().get(path) {
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
      self
        .status
        .index_diff()
        .keys()
        .chain(self.status.workspace_diff().keys()),
    );

    for path in paths {
      ctx.println(format!(
        "{} {}",
        self.status_for(&path, use_color),
        PathBuf::from(path).display()
      ));
    }

    for spec in self.status.untracked().keys() {
      ctx.println(format!("?? {}", PathBuf::from(spec).display()));
    }
  }

  fn print_full(&self, ctx: &Context) {
    self.print_changes(
      ctx,
      "Changes to be committed",
      self.status.index_diff(),
      Color::Green,
    );
    self.print_changes(
      ctx,
      "Changes not staged for commit",
      self.status.workspace_diff(),
      Color::Red,
    );
    self.print_changes(
      ctx,
      "Untracked files",
      self.status.untracked(),
      Color::Red,
    );
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
    if self.status.has_index_changes() {
      return;
    }

    if self.status.has_workspace_changes() {
      ctx.println("no changes added to commit".into())
    } else if self.status.has_workspace_changes() {
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
    tr.repo.index_mut().reload().expect("couldn't reload index");
    tr.run_pidgit(vec!["add", "."]).expect("bad add!");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "D  a/b/3.txt");
  }

  #[test]
  fn deleted_dir_from_index() {
    let tr = new_with_commit();

    tr.rm_rf("a");
    tr.rm_file(".pidgit/index");
    tr.repo.index_mut().reload().expect("couldn't reload index");
    tr.run_pidgit(vec!["add", "."]).expect("bad add!");

    let stdout = tr.run_pidgit(vec!["status", "-s"]).unwrap();
    assert_status(stdout, "D  a/2.txt\nD  a/b/3.txt");
  }
}
