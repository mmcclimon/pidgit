use clap::{App, ArgMatches};
use std::io::Write;

use crate::{find_repo, Repository, Result};

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

pub fn run(_matches: &ArgMatches) -> Result<()> {
  if let Ok(repo) = find_repo() {
    // maybe later: die if we can't initialize a repo from it
    println!(
      "{} already exists, nothing to do!",
      repo.work_tree().display()
    );
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

  println!(
    "initialized empty pidgit repository at {}",
    repo.gitdir().display()
  );

  Ok(())
}
