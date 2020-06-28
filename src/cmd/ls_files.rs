use clap::{App, ArgMatches};
use std::path::PathBuf;

use crate::prelude::*;

pub fn command() -> Command {
  (app, run)
}

pub fn app() -> ClapApp {
  // this doesn't have all the smarts git does, for now
  App::new("ls-files").about("list all the files in the tree")
}

fn run(_matches: &ArgMatches, ctx: &Context) -> Result<()> {
  for key in ctx.repo()?.index().keys() {
    ctx.println(format!("{}", PathBuf::from(key).display()));
  }

  Ok(())
}
