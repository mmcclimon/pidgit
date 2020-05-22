use clap::{App, Arg, ArgMatches};

use crate::object::{Commit, GitObject};
use crate::{find_repo, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("log")
    .about("show commit logs")
    .arg(Arg::with_name("sha").required(true).help("sha to start"))
}

pub fn run(matches: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;

  let object = repo.object_for_sha(matches.value_of("sha").unwrap())?;

  let mut c = Commit::from(object);

  // this is terrible, but expedient
  loop {
    println!("{} {}", &c.sha().hexdigest()[0..8], c.title());
    let mut parents = c.parents(&repo);
    if let Some(parent) = parents.pop() {
      c = parent;
    } else {
      break;
    }
  }

  Ok(())
}
