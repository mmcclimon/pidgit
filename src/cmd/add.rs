use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

use crate::cmd::{Stdout, Writeable};
use crate::index::IndexEntry;
use crate::object::Blob;
use crate::prelude::*;

#[derive(Debug)]
struct Add<W: Writeable> {
  stdout: Stdout<W>,
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("add").about("add file contents to the index").arg(
    Arg::with_name("pathspec")
      .required(true)
      .multiple(true)
      .help("path(s) to add to index"),
  )
}

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(Add { stdout })
}

impl<W: Writeable> Command<W> for Add<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, matches: &ArgMatches) -> Result<()> {
    let repo = util::find_repo()?;
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
