use clap::{App, Arg, ArgMatches};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use crate::{util, Object, PidgitError, Result};

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
    .arg(Arg::with_name("path").required(true).help("path to hash"))
}

pub fn run(matches: &ArgMatches) -> Result<()> {
  // I don't think we even _need_ the repository? Well, at least until we
  // implement -w.
  // let repo = find_repo()?;
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

  let mut content = vec![];

  let mut reader = BufReader::new(File::open(&path)?);
  reader.read_to_end(&mut content)?;

  let sha = util::hash_object(&Object::Blob, &content);
  println!("{}", sha.hexdigest());

  Ok(())
}
