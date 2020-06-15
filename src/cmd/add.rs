use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

use crate::cmd::Context;
use crate::index::IndexEntry;
use crate::object::Blob;
use crate::prelude::*;

#[derive(Debug)]
struct Add;

pub fn new() -> Box<dyn Command> {
  Box::new(Add {})
}

impl Command for Add {
  fn app(&self) -> App<'static, 'static> {
    App::new("add").about("add file contents to the index").arg(
      Arg::with_name("pathspec")
        .required(true)
        .multiple(true)
        .help("path(s) to add to index"),
    )
  }

  fn run(&self, matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;
    let mut index = repo.index()?;

    for raw_path in matches.values_of("pathspec").unwrap() {
      let base = PathBuf::from(raw_path); // .canonicalize()?;

      for path in repo.list_files_from_base(&base)? {
        let entry = IndexEntry::new(&path)?;

        let blob = Blob::from_path(&path)?;
        repo.write_object(&blob)?;

        index.add(entry);
      }
    }

    repo.write_index(&index)?;

    Ok(())
  }
}
