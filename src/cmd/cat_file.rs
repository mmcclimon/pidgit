use clap::{App, Arg, ArgMatches};

use crate::{find_repo, Result};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("cat-file")
    .about("get information about repository objects")
    .arg(
      Arg::with_name("object")
        .required(true)
        .help("object to view"),
    )
}

pub fn run(matches: &ArgMatches) -> Result<()> {
  let _repo = find_repo()?;

  println!("{:#?}", matches);

  Ok(())
}
