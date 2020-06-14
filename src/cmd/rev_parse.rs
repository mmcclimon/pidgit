use clap::{App, Arg, ArgMatches};

use crate::cmd::Stdout;
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

  fn run(&self, matches: &ArgMatches, stdout: &Stdout) -> Result<()> {
    let repo = util::find_repo()?;

    let object = repo.resolve_object(matches.value_of("object").unwrap())?;

    stdout.println(format!("{}", object.get_ref().sha().hexdigest()));

    Ok(())
  }
}
