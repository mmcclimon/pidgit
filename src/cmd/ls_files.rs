use clap::{App, ArgMatches};
use std::path::PathBuf;

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

  fn run(&mut self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    for key in ctx.repo()?.index().keys() {
      ctx.println(format!("{}", PathBuf::from(key).display()));
    }

    Ok(())
  }
}
