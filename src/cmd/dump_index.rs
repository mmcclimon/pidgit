use clap::{App, ArgMatches};

use crate::cmd::Context;
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

  fn run(&self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;
    let index = repo.index();

    ctx.println(format!("{:#?}", index));

    index.write()?;

    Ok(())
  }
}
