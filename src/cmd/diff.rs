use clap::{App, ArgMatches};
use std::cell::Ref;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::cmd::Context;
use crate::index::Index;
use crate::prelude::*;
use crate::repo::{ChangeType, Status};

const NULL_SHA: &str = "0000000000000000000000000000000000000000";
const NULL_PATH: &str = "/dev/null";

#[derive(Debug)]
struct Diff<'r> {
  repo:   &'r Repository,
  status: Status,
  index:  Ref<'r, Index>,
}

#[derive(Debug)]
struct DiffTarget {
  path: PathBuf,
  sha:  String,
  mode: u32,
}

pub fn command() -> Command {
  (app, run)
}

pub fn app() -> ClapApp {
  App::new("diff")
    .about("show changes between commits, commit and working tree, etc.")
}

fn run(_matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;
  let status = repo.status()?;
  let index = repo.index();

  let cmd = Diff {
    repo,
    status,
    index,
  };

  for (path, state) in cmd.status.workspace_diff().iter() {
    match state {
      ChangeType::Modified => cmd.diff_modified(ctx, path),
      ChangeType::Deleted => cmd.diff_deleted(ctx, path),
      _ => println!("{:?}, {:?}", path, state),
    }
  }

  Ok(())
}

impl<'r> Diff<'r> {
  fn print_diff(&self, ctx: &Context, mut a: DiffTarget, mut b: DiffTarget) {
    if a.sha == b.sha && a.mode == b.mode {
      return;
    }

    a.path = a.with_prefix("a");
    b.path = b.with_prefix("b");

    ctx.println(format!(
      "diff --git {} {}",
      a.path.display(),
      b.path.display()
    ));

    // mode
    if b.is_null() {
      ctx.println(format!("deleted file mode {:0o}", a.mode));
    } else if a.mode != b.mode {
      ctx.println(format!("old mode {:0o}", a.mode));
      ctx.println(format!("new mode {:0o}", b.mode));
    }

    // content
    if a.sha == b.sha {
      return;
    }

    let mode_str = if a.mode == b.mode {
      format!(" {:0o}", a.mode)
    } else {
      "".to_string()
    };

    ctx.println(format!(
      "index {}..{}{}",
      &a.sha[0..8],
      &b.sha[0..8],
      mode_str,
    ));

    ctx.println(format!("--- {}", a.diff_path().display()));
    ctx.println(format!("+++ {}", a.diff_path().display()));
  }

  fn target_from_index(&self, path: &OsString) -> DiffTarget {
    let entry = self.index.entry_for(path).expect("missing index entry!");
    DiffTarget::from(entry)
  }

  fn target_from_file(&self, path: &OsString) -> DiffTarget {
    use std::os::unix::fs::PermissionsExt;

    let blob = self
      .repo
      .workspace()
      .read_blob(path)
      .expect("could not create blob");
    let stat = self.status.stat_for(path).expect("missing stat");

    DiffTarget {
      path: path.into(),
      sha:  blob.sha().hexdigest(),
      mode: stat.permissions().mode(),
    }
  }

  fn diff_modified(&self, ctx: &Context, path: &OsString) {
    let a = self.target_from_index(path);
    let b = self.target_from_file(path);
    self.print_diff(ctx, a, b);
  }

  fn diff_deleted(&self, ctx: &Context, path: &OsString) {
    let a = self.target_from_index(path);
    let b = DiffTarget::null(path);
    self.print_diff(ctx, a, b);
  }
}

impl From<&crate::index::IndexEntry> for DiffTarget {
  fn from(entry: &crate::index::IndexEntry) -> Self {
    Self {
      path: entry.name.clone().into(),
      sha:  entry.sha.clone(),
      mode: entry.mode(),
    }
  }
}

impl DiffTarget {
  fn null(path: &OsString) -> Self {
    Self {
      path: path.into(),
      sha:  NULL_SHA.to_string(),
      mode: 0,
    }
  }

  fn with_prefix(&self, prefix: &str) -> PathBuf {
    PathBuf::from(prefix).join(&self.path)
  }

  fn is_null(&self) -> bool {
    self.mode == 0
  }

  fn diff_path(&self) -> PathBuf {
    if self.is_null() {
      NULL_PATH.into()
    } else {
      self.path.clone()
    }
  }
}
