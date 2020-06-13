use clap::{App, ArgMatches};

use crate::cmd::{Stdout, Writeable};
use crate::prelude::*;

#[derive(Debug)]
struct DumpIndex<W: Writeable> {
  stdout: Stdout<W>,
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("dump-index").about("dump the index file (just for debugging)")
}

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(DumpIndex { stdout })
}

impl<W: Writeable> Command<W> for DumpIndex<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, _matches: &ArgMatches) -> Result<()> {
    let repo = util::find_repo()?;
    let index = repo.index()?;

    self.println(format!("{:#?}", index));

    index.write()?;

    Ok(())
  }
}
