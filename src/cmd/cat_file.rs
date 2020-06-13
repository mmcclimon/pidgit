use clap::{App, Arg, ArgMatches};

use crate::prelude::*;

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
      Arg::with_name("debug")
        .long("debug")
        .help("dump the object's data structure (for debugging rust internals)"),
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

pub fn run<W>(m: &ArgMatches, stdout: &mut W) -> Result<()>
where
  W: std::io::Write,
{
  let repo = util::find_repo()?;

  let object = repo.resolve_object(m.value_of("object").unwrap())?;
  let inner = object.into_inner();

  match inner {
    _ if m.is_present("type") => writeln!(stdout, "{}", inner.type_str())?,
    _ if m.is_present("size") => writeln!(stdout, "{}", inner.size())?,
    _ if m.is_present("debug") => writeln!(stdout, "{:#?}", inner)?,
    _ if m.is_present("pretty") => {
      stdout.write_all(&inner.pretty())?;
      stdout.write(b"\n")?;
    },
    _ => writeln!(stdout, "{} {}", inner.type_str(), inner.size())?,
  };

  Ok(())
}
