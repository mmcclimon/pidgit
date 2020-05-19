use clap::{App, Arg, ArgMatches};

use crate::{find_repo, Result};

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

pub fn run(m: &ArgMatches) -> Result<()> {
  let repo = find_repo()?;

  let object = repo.object_for_sha(m.value_of("object").unwrap())?;

  match object {
    _ if m.is_present("type") => println!("{}", object.string_type()),
    _ if m.is_present("size") => println!("{}", object.size()),
    _ if m.is_present("pretty") => println!("{}", object.inflate().pretty()?),
    _ => println!("{} {}", object.string_type(), object.size()),
  };

  Ok(())
}
