use clap::{App, ArgMatches};
use std::fs::DirBuilder;

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
//    index  ??
//    objects/
//    refs/
//      heads/
//      tags/
//      remotes/
//

pub fn run(_matches: &ArgMatches) {
  let gitdir = std::env::current_dir().unwrap().join(".pidgit");

  if gitdir.is_dir() {
    // maybe later: die if we can't initialize a repo from it
    println!("{} already exists, nothing to do!", gitdir.display());
    return;
  }

  // I think for now, I'm just going to create these imperatively, and later,
  // once the repository is abstracted a bit, rework this to say something like
  // Repository.create_new()
  DirBuilder::new().create(&gitdir).unwrap();

  // HEAD
  std::fs::write(gitdir.join("HEAD"), HEAD).unwrap();

  // config
  std::fs::write(gitdir.join("config"), CONFIG).unwrap();

  // index
  std::fs::write(gitdir.join("index"), "").unwrap();

  // object dir
  DirBuilder::new().create(gitdir.join("objects")).unwrap();

  // refs
  let refdir = gitdir.join("refs");
  DirBuilder::new().create(&refdir).unwrap();
  DirBuilder::new().create(refdir.join("heads")).unwrap();
  DirBuilder::new().create(refdir.join("tags")).unwrap();
  DirBuilder::new().create(refdir.join("remotes")).unwrap();

  println!(
    "initialized empty pidgit repository at {}",
    gitdir.display()
  );
}
