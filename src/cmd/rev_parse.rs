use clap::{App, Arg, ArgMatches};

use crate::prelude::*;

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("rev-parse")
    .about("pick out and massage parameters")
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

  writeln!(stdout, "{}", object.get_ref().sha().hexdigest())?;

  Ok(())
}
