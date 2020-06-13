use chrono::Local;
use clap::{App, Arg, ArgMatches};

use crate::object::{Commit, Person, Tree};
use crate::prelude::*;

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

pub fn run<W>(matches: &ArgMatches, stdout: &mut W) -> Result<()>
where
  W: std::io::Write,
{
  let repo = util::find_repo()?;

  // first pass, will improve later
  let who = Person {
    name:  std::env::var("GIT_AUTHOR_NAME").unwrap(),
    email: std::env::var("GIT_AUTHOR_EMAIL").unwrap(),
  };

  let head = repo.resolve_object("HEAD")?.into_inner();

  let now = Local::now();
  let fixed = now.with_timezone(now.offset());

  let mut msg = matches.value_of("message").unwrap().to_string();

  if !msg.ends_with("\n") {
    msg.push_str("\n");
  }

  // let tree = repo.as_tree()?;
  let index = repo.index()?;
  let tree = Tree::from(&index);

  let commit = Commit {
    tree:           tree.sha().hexdigest(),
    parent_shas:    vec![head.sha().hexdigest()],
    author:         who.clone(),
    author_date:    fixed,
    committer:      who,
    committer_date: fixed,
    message:        msg,
    content:        None,
  };

  // we write the tree, then write the commit.
  // ...obviously, this should be improved a lot, as it's destructive.
  repo.write_tree(&tree)?;
  repo.write_object(&commit)?;
  repo.update_head(&commit.sha())?;

  writeln!(
    stdout,
    "[{}] {}",
    &commit.sha().hexdigest()[0..8],
    commit.title()
  )?;

  Ok(())
}
