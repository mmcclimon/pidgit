use chrono::Local;
use clap::{App, Arg, ArgMatches};
use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::object::{Commit, GitObject, Person};
use crate::{find_repo, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("commit")
    .about("record changes to the repository")
    .arg(
      Arg::with_name("message")
        .short("m")
        .long("message")
        .takes_value(true)
        .value_name("msg")
        .required(true)
        .help("use this as the message"),
    )
}

pub fn run(matches: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;

  // first pass, will improve later
  let who = Person {
    name:  std::env::var("GIT_AUTHOR_NAME").unwrap(),
    email: std::env::var("GIT_AUTHOR_EMAIL").unwrap(),
  };

  let head = repo.resolve_object("HEAD")?.into_inner();

  let now = Local::now();
  let fixed = now.with_timezone(now.offset());

  let msg = matches.value_of("message").unwrap();

  let tree = repo.as_tree()?;

  let commit = Commit {
    tree:           tree.sha().hexdigest(),
    parent_shas:    vec![head.sha().hexdigest()],
    author:         who.clone(),
    author_date:    fixed,
    committer:      who,
    committer_date: fixed,
    message:        msg.to_string(),
    content:        None,
  };

  // we write the tree, then write the commit.
  // ...obviously, this should be improved a lot, as it's destructive.
  // repo.write_object(&tree)?;
  repo.write_tree()?;
  repo.write_object(&commit)?;

  // update our ref
  let mut f = OpenOptions::new()
    .write(true)
    .open(repo.git_dir().join("refs/heads/master"))?;

  f.write_all(format!("{}\n", commit.sha().hexdigest()).as_bytes())?;

  println!("[{}] {}", &commit.sha().hexdigest()[0..8], commit.title());

  Ok(())
}
