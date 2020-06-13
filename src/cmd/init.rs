use clap::{App, ArgMatches};

use crate::cmd::{Stdout, Writeable};
use crate::prelude::*;

#[derive(Debug)]
struct Init<W: Writeable> {
  stdout: Stdout<W>,
}

const HEAD: &'static str = "ref: refs/heads/master\n";

const CONFIG: &'static str = "\
[core]
	repositoryformatversion = 0
	filemode = true
	bare = false
	logallrefupdates = true
	ignorecase = true
	precomposeunicode = true
";

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("init").about("initialize a pidgit directory")
}

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(Init { stdout })
}

// We need to make, inside the current directory:
// .pidgit/
//    HEAD
//    config
//    index   <-- no, for now.
//    objects/
//    refs/
//      heads/
//      tags/
//      remotes/
//

impl<W: Writeable> Command<W> for Init<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, _matches: &ArgMatches) -> Result<()> {
    if let Ok(repo) = util::find_repo() {
      // maybe later: die if we can't initialize a repo from it
      self.println(format!(
        "{} already exists, nothing to do!",
        repo.work_tree().display()
      ));
      return Ok(());
    }

    let pwd = std::env::current_dir()?;
    let repo = Repository::create_empty(&pwd)?;

    // HEAD
    let mut head = repo.create_file("HEAD")?;
    head.write_all(HEAD.as_bytes())?;

    // config
    let mut config = repo.create_file("config")?;
    config.write_all(CONFIG.as_bytes())?;

    // object dir
    repo.create_dir("objects")?;

    // refs
    repo.create_dir("refs/heads")?;
    repo.create_dir("refs/tags")?;
    repo.create_dir("refs/remotes")?;

    self.println(format!(
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

  // eventually, all this stuff will be listed into some common module

  // make a tempdir and cd to it
  fn cd_temp() -> Result<TempDir> {
    let dir = TempDir::new()?;
    std::env::set_current_dir(&dir.path())?;
    Ok(dir)
  }

  #[test]
  fn init_help() {
    let res = run_pidgit(vec!["init", "--help"]);

    if let Err(PidgitError::Clap(err)) = res {
      let help = err.message;
      assert!(help.contains("pidgit init"));
      assert!(help.contains("initialize a pidgit directory"));
    } else {
      panic!("help not displayed");
    }
  }

  #[test]
  #[serial]
  fn init_dot_pidgit_exists() -> Result<()> {
    let dir = cd_temp()?;
    let pidgit_path = dir.child(".pidgit");
    pidgit_path.create_dir_all()?;
    pidgit_path.assert(predicate::path::is_dir());

    let stdout = run_pidgit(vec!["init"])?;
    assert!(stdout.contains("nothing to do"));

    Ok(())
  }

  #[test]
  #[serial]
  fn init_create_dir() -> Result<()> {
    use predicate::path::is_dir;
    use predicate::str::contains;

    let dir = cd_temp()?;
    let stdout = run_pidgit(vec!["init"])?;
    assert!(stdout.contains("initialized empty pidgit repository"));

    let ppath = dir.child(".pidgit");

    ppath.child("HEAD").assert(contains("refs/heads/master"));
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
