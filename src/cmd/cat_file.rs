use clap::{App, Arg, ArgMatches};

use crate::cmd::{Stdout, Writeable};
use crate::prelude::*;

#[derive(Debug)]
struct CatFile<W: Writeable> {
  stdout: Stdout<W>,
}

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

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(CatFile { stdout })
}

impl<W: Writeable> Command<W> for CatFile<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, m: &ArgMatches) -> Result<()> {
    let repo = util::find_repo()?;

    let object = repo.resolve_object(m.value_of("object").unwrap())?;
    let inner = object.into_inner();

    match inner {
      _ if m.is_present("type") => self.println(format!("{}", inner.type_str())),
      _ if m.is_present("size") => self.println(format!("{}", inner.size())),
      _ if m.is_present("debug") => self.println(format!("{:#?}", inner)),
      _ if m.is_present("pretty") => {
        self.stdout.println_raw(&inner.pretty())?;
      },
      _ => self.println(format!("{} {}", inner.type_str(), inner.size())),
    };

    Ok(())
  }
}
