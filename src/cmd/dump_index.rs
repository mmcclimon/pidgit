use clap::{App, ArgMatches};

use crate::cmd::Stdout;
use crate::prelude::*;

#[derive(Debug)]
struct DumpIndex;

pub fn new() -> Box<dyn Command> {
  Box::new(DumpIndex {})
}

impl Command for DumpIndex {
  fn app(&self) -> App<'static, 'static> {
    App::new("dump-index").about("dump the index file (just for debugging)")
  }

  fn run(&self, _matches: &ArgMatches, stdout: &Stdout) -> Result<()> {
    let repo = util::find_repo()?;
    let index = repo.index()?;

    stdout.println(format!("{:#?}", index));

    index.write()?;

    Ok(())
  }
}
