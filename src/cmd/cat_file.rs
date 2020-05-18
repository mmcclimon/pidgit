use clap::{App, Arg, ArgMatches};

use crate::{find_repo, Object, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("cat-file")
    .about("get information about repository objects")
    .arg(
      Arg::with_name("type")
        .short("t")
        .long("type")
        .conflicts_with("size")
        .help("show object's type, instead of its content"),
    )
    .arg(
      Arg::with_name("size")
        .short("s")
        .long("size")
        .conflicts_with("type")
        .help("show object's size, instead of its content"),
    )
    .arg(
      Arg::with_name("object")
        .required(true)
        .help("object to view"),
    )
}

pub fn run(matches: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;

  let obj = repo.object_for_sha(matches.value_of("object").unwrap())?;

  if let Object::NotFound = obj {
    println!("object not found!");
    return Ok(());
  }

  if matches.is_present("type") {
    let s = match obj {
      Object::Blob(_) => "blob",
      Object::Commit(_) => "commit",
      Object::Tag(_) => "tag",
      Object::Tree(_) => "tree",
      Object::Generic => "unknown type!",
      _ => unreachable!(),
    };

    println!("{}", s);
    return Ok(());
  }

  if matches.is_present("size") {
    let size = match obj {
      Object::Blob(size) => size,
      Object::Commit(size) => size,
      Object::Tag(size) => size,
      Object::Tree(size) => size,
      Object::Generic => 0,
      _ => unreachable!(),
    };

    println!("{}", size);
    return Ok(());
  }

  println!("got object {:?}", obj);

  Ok(())
}
