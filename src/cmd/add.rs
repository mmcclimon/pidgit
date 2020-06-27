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
    let workspace = repo.workspace();

    let mut index = repo.index_mut();

    for raw_path in matches.values_of("pathspec").unwrap() {
      let pb = PathBuf::from(raw_path);
      let base = if pb.is_absolute() {
        pb.canonicalize()?
      } else {
        ctx.pwd.join(raw_path).canonicalize()?
      };

      for path in workspace.list_files_from_base(&base)? {
        let entry =
          IndexEntry::new(path.clone(), &workspace.canonicalize(&path))?;

        let blob = Blob::from_path(&ctx.pwd.join(&path))?;
        repo.write_object(&blob)?;

        index.add(entry);
      }
    }

    index.write()?;

    Ok(())
  }
}
