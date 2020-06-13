use clap::{App, ArgMatches};

use crate::cmd::{Stdout, Writeable};
use crate::prelude::*;

#[derive(Debug)]
struct LsFiles<W: Writeable> {
  stdout: Stdout<W>,
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
  // this doesn't have all the smarts git does, for now
  App::new("ls-files").about("list all the files in the tree")
}

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(LsFiles { stdout })
}

impl<W: Writeable> Command<W> for LsFiles<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, _matches: &ArgMatches) -> Result<()> {
    let repo = util::find_repo()?;

    for entry in repo.list_files()? {
      self.println(format!("{}", entry.display()));
    }

    Ok(())
  }
}
