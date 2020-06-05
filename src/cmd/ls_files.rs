use clap::{App, ArgMatches};

use crate::object::TreeEntry;
use crate::prelude::*;

pub fn app<'a, 'b>() -> App<'a, 'b> {
  // this doesn't have all the smarts git does, for now
  App::new("ls-files").about("list all the files in the tree")
}

pub fn run(_matches: &ArgMatches) -> Result<()> {
  let repo = util::find_repo()?;

  for entry in repo.list_files()? {
    let te = TreeEntry::from_path(&entry);
    println!("{:?}", te);
  }

  Ok(())
}
