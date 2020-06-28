use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

use crate::cmd::Context;
use crate::object::Blob;
use crate::prelude::*;

#[derive(Debug)]
struct HashObject;

pub fn new() -> Box<dyn Command> {
  Box::new(HashObject {})
}

impl Command for HashObject {
  fn app(&self) -> App<'static, 'static> {
    App::new("hash-object")
      .about("compute object id and optionally create blob from file")
      .arg(
        Arg::with_name("type")
          .short("t")
          .long("type")
          .default_value("blob")
          .help("specify the object type"),
      )
      .arg(
        Arg::with_name("write")
          .short("w")
          .long("write")
          .help("write the object into the object database"),
      )
      .arg(Arg::with_name("path").required(true).help("path to hash"))
  }

  fn run(&mut self, matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo();

    if repo.is_err() && matches.is_present("write") {
      return repo.map(|_| ());
    }

    let path = PathBuf::from(matches.value_of("path").unwrap());

    if !path.exists() {
      return Err(PidgitError::Generic(format!(
        "cannot open {}: no such file or directory",
        path.display()
      )));
    }

    if !path.is_file() {
      return Err(PidgitError::Generic(format!(
        "unable to hash {}",
        path.display()
      )));
    }

    let blob = Blob::from_path(&path)?;

    if matches.is_present("write") {
      repo.unwrap().write_object(&blob)?;
    }

    ctx.println(format!("{}", blob.sha().hexdigest()));

    Ok(())
  }
}
