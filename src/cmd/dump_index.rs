use clap::{App, ArgMatches};

use crate::prelude::*;

pub fn command() -> Command {
  (app, run)
}

pub fn app() -> ClapApp {
  App::new("dump-index").about("dump the index file (just for debugging)")
}

pub fn run(_matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;
  let index = repo.index();

  ctx.println(format!("{:#?}", index));

  index.write()?;

  Ok(())
}
