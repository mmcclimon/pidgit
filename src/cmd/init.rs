use clap::{App, ArgMatches};

use crate::cmd::Context;
use crate::prelude::*;

#[derive(Debug)]
struct Init;

pub fn new() -> Box<dyn Command> {
  Box::new(Init {})
}

impl Command for Init {
  fn app(&self) -> App<'static, 'static> {
    App::new("init").about("initialize a pidgit directory")
  }

  fn run(&self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    if let Ok(repo) = ctx.repo() {
      // maybe later: die if we can't initialize a repo from it
      ctx.println(format!(
        "{} already exists, nothing to do!",
        repo.work_tree().display()
      ));
      return Ok(());
    }

    let pwd = std::env::current_dir()?;
    let repo = Repository::create_empty(&pwd)?;

    ctx.println(format!(
      "initialized empty pidgit repository at {}",
      repo.git_dir().display()
    ));

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::test_prelude::*;
  type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

  #[test]
  fn init_help() {
    let tr = new_empty_repo();
    let res = tr.run_pidgit(vec!["init", "--help"]);

    if let Err(PidgitError::Clap(err)) = res {
      let help = err.message;
      assert!(help.contains("pidgit init"));
      assert!(help.contains("initialize a pidgit directory"));
    } else {
      panic!("help not displayed");
    }
  }

  #[test]
  fn init_dot_pidgit_exists() -> Result<()> {
    let tr = new_empty_repo();
    let stdout = tr.run_pidgit(vec!["init"])?;
    assert!(stdout.contains("nothing to do"));

    Ok(())
  }

  #[test]
  #[serial] // must be run in isolation because it relies on $PWD
  fn init_create_dir() -> Result<()> {
    use predicate::path::is_dir;
    use predicate::str::contains;
    use std::io::Cursor;

    let dir = tempdir();
    std::env::set_current_dir(&dir.path())?;

    // this is silly, but this test is also like, the only place we need to run
    // outside of a repository
    let app = pidgit::new();
    let mut stdout = Cursor::new(vec![]);
    let matches = app.clap_app().get_matches_from_safe(&["pidgit", "init"])?;

    app.dispatch(&matches, None, &mut stdout, std::env::current_dir()?)?;

    let out = String::from_utf8(stdout.into_inner())?;
    assert!(out.contains("initialized empty pidgit repository"));

    let ppath = dir.child(".pidgit");

    ppath.child("HEAD").assert(contains("refs/heads/main"));
    ppath
      .child("config")
      .assert(contains("repositoryformatversion = 0"));

    ppath.child("objects").assert(is_dir());
    ppath.child("refs").child("heads").assert(is_dir());
    ppath.child("refs").child("tags").assert(is_dir());
    ppath.child("refs").child("remotes").assert(is_dir());

    Ok(())
  }
}
