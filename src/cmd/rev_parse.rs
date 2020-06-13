use clap::{App, Arg, ArgMatches};

use crate::cmd::{Stdout, Writeable};
use crate::prelude::*;

#[derive(Debug)]
struct RevParse<W: Writeable> {
  stdout: Stdout<W>,
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("rev-parse")
    .about("pick out and massage parameters")
    .arg(
      Arg::with_name("object")
        .required(true)
        .help("object to view"),
    )
}

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(RevParse { stdout })
}

impl<W: Writeable> Command<W> for RevParse<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, matches: &ArgMatches) -> Result<()> {
    let repo = util::find_repo()?;

    let object = repo.resolve_object(matches.value_of("object").unwrap())?;

    self.println(format!("{}", object.get_ref().sha().hexdigest()));

    Ok(())
  }
}
