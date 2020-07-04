use clap::{App, Arg, ArgMatches};
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
    .arg(
      Arg::with_name("cached")
        .long("cached")
        .alias("staged")
        .help("view staged changes"),
    )
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;
  let status = repo.status()?;
  let index = repo.index();

  let cmd = Diff {
    repo,
    status,
    index,
  };

  if matches.is_present("cached") {
    cmd.print_index_diff(ctx);
  } else {
    cmd.print_workspace_diff(ctx);
  }

  Ok(())
}

impl<'r> Diff<'r> {
  fn print_workspace_diff(&self, ctx: &Context) {
    for (path, state) in self.status.workspace_diff().iter() {
      match state {
        ChangeType::Modified => self.diff_modified(ctx, path),
        ChangeType::Deleted => self.diff_deleted(ctx, path),
        _ => println!("{:?}, {:?}", path, state),
      }
    }
  }

  fn print_index_diff(&self, ctx: &Context) {
    for (path, state) in self.status.index_diff().iter() {
      match state {
        ChangeType::Modified => self.idx_diff_modified(ctx, path),
        ChangeType::Deleted => self.idx_diff_deleted(ctx, path),
        _ => println!("{:?}, {:?}", path, state),
      }
    }
  }

  fn print_diff(&self, ctx: &Context, mut a: DiffTarget, mut b: DiffTarget) {
    println!("print diff");
    if a.sha == b.sha && a.mode == b.mode {
      println!("shas and modes match");
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

  fn target_from_head(&self, path: &OsString) -> DiffTarget {
    let entry = self
      .status
      .head_diff()
      .get(path)
      .expect("missing index entry!");
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

  fn idx_diff_modified(&self, ctx: &Context, path: &OsString) {
    let a = self.target_from_head(path);
    let b = self.target_from_index(path);
    self.print_diff(ctx, a, b);
  }

  fn diff_modified(&self, ctx: &Context, path: &OsString) {
    let a = self.target_from_index(path);
    let b = self.target_from_file(path);
    self.print_diff(ctx, a, b);
  }

  fn idx_diff_deleted(&self, ctx: &Context, path: &OsString) {
    let a = self.target_from_head(path);
    let b = DiffTarget::null(path);
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

// path: PathBuf,
// mode: Mode,
// sha:  String,
impl From<&crate::object::PathEntry> for DiffTarget {
  fn from(entry: &crate::object::PathEntry) -> Self {
    Self {
      path: entry.path.clone(),
      sha:  entry.sha.clone(),
      mode: entry.mode().into(),
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
