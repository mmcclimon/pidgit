use clap::{App, ArgMatches};
use std::io::Write;

use crate::{find_repo, Repository};

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

pub fn run(_matches: &ArgMatches) {
  if let Some(repo) = find_repo() {
    // maybe later: die if we can't initialize a repo from it
    println!(
      "{} already exists, nothing to do!",
      repo.work_tree().display()
    );
    return;
  }

  let pwd = std::env::current_dir().unwrap();
  let repo = Repository::create_empty(&pwd);

  // HEAD
  let mut head = repo.create_file("HEAD").unwrap();
  head.write_all(HEAD.as_bytes()).unwrap();

  // config
  let mut config = repo.create_file("config").unwrap();
  config.write_all(CONFIG.as_bytes()).unwrap();

  // object dir
  repo.create_dir("objects").unwrap();

  // refs
  repo.create_dir("refs/heads").unwrap();
  repo.create_dir("refs/tags").unwrap();
  repo.create_dir("refs/remotes").unwrap();

  println!(
    "initialized empty pidgit repository at {}",
    repo.gitdir().display()
  );
}
