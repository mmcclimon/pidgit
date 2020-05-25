use clap::{App, Arg, ArgMatches};

use crate::{find_repo, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("rev-parse")
    .about("pick out and massage parameters")
    .arg(
      Arg::with_name("object")
        .required(true)
        .help("object to view"),
    )
}

pub fn run(m: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;

  let object = repo.resolve_object(m.value_of("object").unwrap())?;

  println!("{}", object.get_ref().sha().hexdigest());

  Ok(())
}
