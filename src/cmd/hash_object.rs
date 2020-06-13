use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

use crate::cmd::{Stdout, Writeable};
use crate::object::Blob;
use crate::prelude::*;

#[derive(Debug)]
struct HashObject<W: Writeable> {
  stdout: Stdout<W>,
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
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

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(HashObject { stdout })
}

impl<W: Writeable> Command<W> for HashObject<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, matches: &ArgMatches) -> Result<()> {
    let repo = util::find_repo();

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

    self.println(format!("{}", blob.sha().hexdigest()));

    Ok(())
  }
}
