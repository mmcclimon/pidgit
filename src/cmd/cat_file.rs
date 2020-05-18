use clap::{App, Arg, ArgMatches};
use std::io::prelude::*;
use std::io::Cursor;

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
      Arg::with_name("pretty")
        .short("p")
        .long("pretty")
        .help("pretty-print object's content"),
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
      Object::Blob(_, _) => "blob",
      Object::Commit(_, _) => "commit",
      Object::Tag(_, _) => "tag",
      Object::Tree(_, _) => "tree",
      _ => unreachable!(),
    };

    println!("{}", s);
    return Ok(());
  }

  if matches.is_present("size") {
    let size = match obj {
      Object::Blob(size, _) => size,
      Object::Commit(size, _) => size,
      Object::Tag(size, _) => size,
      Object::Tree(size, _) => size,
      _ => unreachable!(),
    };

    println!("{}", size);
    return Ok(());
  }

  if matches.is_present("pretty") {
    let content = match obj {
      Object::Blob(_, c) => c,
      Object::Commit(_, c) => c,
      Object::Tag(_, c) => c,
      Object::Tree(_, content) => {
        // a tree is made of entries, where each entry entry is:
        // mode filename NULL 20-bytes-of-sha
        let mut reader = Cursor::new(&content);
        let len = reader.get_ref().len();

        while (reader.position() as usize) < len {
          let mut mode = vec![];
          reader.read_until(b' ', &mut mode)?;
          mode.pop();

          let mut filename = vec![];
          reader.read_until(b'\0', &mut filename)?;
          filename.pop();

          let mut sha = [0u8; 20];
          reader.read_exact(&mut sha)?;

          println!(
            "{:0>6} {}    {}",
            String::from_utf8(mode)?,
            hex::encode(sha),
            String::from_utf8(filename)?,
          );
        }

        return Ok(());
      },
      _ => unreachable!(),
    };

    println!("{:?}", String::from_utf8_lossy(&content));
    return Ok(());
  }

  // this is silly behavior of git cat-file, but hey.
  println!("{}", matches.usage());

  Ok(())
}
