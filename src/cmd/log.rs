use clap::{App, Arg, ArgMatches};

use crate::object::{GitObject, Object};
use crate::{find_repo, PidgitError, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("log").about("show commit logs").arg(
    Arg::with_name("ref")
      .default_value("HEAD")
      .help("ref to start"),
  )
}

pub fn run(matches: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;

  let to_find = matches.value_of("ref").unwrap();
  let object = repo.resolve_object(to_find)?;

  let mut c = match object {
    Object::Commit(commit) => commit,
    _ => {
      return Err(PidgitError::Generic(format!(
        "ref {} is not a commit!",
        to_find,
      )))
    },
  };

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
