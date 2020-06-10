use clap::{App, ArgMatches};
use std::io::prelude::*;

use crate::object::{PathEntry, Tree};
use crate::prelude::*;

pub fn app<'a, 'b>() -> App<'a, 'b> {
  // this doesn't have all the smarts git does, for now
  App::new("ls-files").about("list all the files in the tree")
}

pub fn run(_matches: &ArgMatches) -> Result<()> {
  let repo = util::find_repo()?;

  let entries = repo
    .list_files()?
    .iter()
    .filter_map(|entry| PathEntry::from_path(&entry).ok())
    .collect::<Vec<_>>();

  let t = Tree::build(entries);

  t.traverse(&|tree| {
    let mut stdout = std::io::stdout();
    stdout.write_all(&tree.pretty()).unwrap();
    stdout.write(b"\n\n").unwrap();
  });

  // println!("{:#?}", t);

  Ok(())
}
