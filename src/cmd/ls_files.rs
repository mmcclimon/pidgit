use clap::{App, ArgMatches};

use crate::prelude::*;

pub fn app<'a, 'b>() -> App<'a, 'b> {
  // this doesn't have all the smarts git does, for now
  App::new("ls-files").about("list all the files in the tree")
}

pub fn run<W>(_matches: &ArgMatches, stdout: &mut W) -> Result<()>
where
  W: std::io::Write,
{
  let repo = util::find_repo()?;

  for entry in repo.list_files()? {
    writeln!(stdout, "{}", entry.display())?;
  }

  Ok(())
}
