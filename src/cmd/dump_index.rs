use clap::{App, ArgMatches};

use crate::prelude::*;

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("dump-index").about("dump the index file (just for debugging)")
}

pub fn run<W>(_matches: &ArgMatches, stdout: &mut W) -> Result<()>
where
  W: std::io::Write,
{
  let repo = util::find_repo()?;
  let index = repo.index()?;

  writeln!(stdout, "{:#?}", index)?;

  index.write()?;

  Ok(())
}
