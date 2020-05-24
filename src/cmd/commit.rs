use clap::{App, Arg, ArgMatches};

use crate::{find_repo, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("commit")
    .about("record changes to the repository")
    .arg(
      Arg::with_name("message")
        .short("m")
        .long("message")
        .takes_value(true)
        .value_name("msg")
        .help("use this as the message"),
    )
}

pub fn run(_matches: &ArgMatches) -> Result<()> {
  let _repo = find_repo()?;

  Ok(())
}
