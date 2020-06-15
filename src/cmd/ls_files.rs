use clap::{App, ArgMatches};

use crate::cmd::Context;
use crate::prelude::*;

#[derive(Debug)]
struct LsFiles;

pub fn new() -> Box<dyn Command> {
  Box::new(LsFiles {})
}

impl Command for LsFiles {
  fn app(&self) -> App<'static, 'static> {
    // this doesn't have all the smarts git does, for now
    App::new("ls-files").about("list all the files in the tree")
  }

  fn run(&self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    for entry in ctx.repo()?.list_files()? {
      ctx.println(format!("{}", entry.display()));
    }

    Ok(())
  }
}
