use clap::{App, ArgMatches};

use crate::{find_repo, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("dump-index").about("dump the index file (just for debugging)")
}

pub fn run(_matches: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;
  let index = repo.index()?;

  println!("{:#?}", index);

  Ok(())
}