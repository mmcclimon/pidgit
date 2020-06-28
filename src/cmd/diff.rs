use clap::{App, ArgMatches};
use std::cell::Ref;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::cmd::Context;
use crate::index::Index;
use crate::prelude::*;
use crate::repo::{ChangeType, Status};

#[derive(Debug)]
struct Diff<'r> {
  repo:   &'r Repository,
  status: Status,
  index:  Ref<'r, Index>,
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
      ChangeType::Modified => cmd.diff_file_modified(ctx, path)?,
      _ => println!("{:?}, {:?}", path, state),
    }
    let _entry = cmd.index.entry_for(path).unwrap();
  }

  Ok(())
}

impl<'r> Diff<'r> {
  fn diff_file_modified(&self, ctx: &Context, path: &OsString) -> Result<()> {
    let a = self.index.entry_for(path).expect("missing index entry!");
    let a_path = PathBuf::from("a").join(path);
    let a_sha = &a.sha;
    let a_mode = a.mode();

    use std::os::unix::fs::PermissionsExt;

    let b = self.repo.workspace().read_blob(path)?;
    let b_path = PathBuf::from("b").join(path);
    let b_sha = &b.sha().hexdigest();
    let b_mode = self
      .status
      .stat_for(path)
      .expect("missing stat")
      .permissions()
      .mode();

    ctx.println(format!(
      "diff --git {} {}",
      a_path.display(),
      b_path.display()
    ));

    if a_mode != b_mode {
      ctx.println(format!("old mode {:0o}", a_mode));
      ctx.println(format!("new mode {:0o}", b_mode));
    }

    if a_sha == b_sha {
      return Ok(());
    }

    let mode_str = if a_mode == b_mode {
      format!(" {:0o}", a_mode)
    } else {
      "".to_string()
    };

    ctx.println(format!(
      "index {}..{}{}",
      &a_sha[0..8],
      &b_sha[0..8],
      mode_str,
    ));

    ctx.println(format!("--- {}", a_path.display()));
    ctx.println(format!("+++ {}", b_path.display()));

    Ok(())
  }
}
