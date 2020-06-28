use clap::{App, Arg, ArgMatches};

use crate::cmd::Context;
use crate::prelude::*;

#[derive(Debug)]
struct RevParse;

pub fn new() -> Box<dyn Command> {
  Box::new(RevParse {})
}

impl Command for RevParse {
  fn app(&self) -> App<'static, 'static> {
    App::new("rev-parse")
      .about("pick out and massage parameters")
      .arg(
        Arg::with_name("object")
          .required(true)
          .help("object to view"),
      )
  }

  fn run(&mut self, matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;

    let object = repo.resolve_object(matches.value_of("object").unwrap())?;

    ctx.println(format!("{}", object.get_ref().sha().hexdigest()));

    Ok(())
  }
}
