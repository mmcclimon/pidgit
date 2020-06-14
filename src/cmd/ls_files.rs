use clap::{App, ArgMatches};

use crate::cmd::Stdout;
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

  fn run(&self, _matches: &ArgMatches, stdout: &Stdout) -> Result<()> {
    let repo = util::find_repo()?;

    for entry in repo.list_files()? {
      stdout.println(format!("{}", entry.display()));
    }

    Ok(())
  }
}
