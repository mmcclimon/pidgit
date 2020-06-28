use clap::{App, Arg, ArgMatches};

use crate::prelude::*;

pub fn command() -> Command {
  (app, run)
}

pub fn app() -> ClapApp {
  App::new("rev-parse")
    .about("pick out and massage parameters")
    .arg(
      Arg::with_name("object")
        .required(true)
        .help("object to view"),
    )
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;

  let object = repo.resolve_object(matches.value_of("object").unwrap())?;

  ctx.println(format!("{}", object.get_ref().sha().hexdigest()));

  Ok(())
}
