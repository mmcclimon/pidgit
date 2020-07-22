use clap::{App, Arg, ArgMatches};

use crate::prelude::*;
use crate::util;

pub fn command() -> Command {
  (app, run)
}

fn app() -> ClapApp {
  App::new("check-ref-format")
    .about("ensure that a reference name is well formed")
    .arg(
      Arg::with_name("normalize")
        .long("normalize")
        .help("collapse slashes"),
    )
    .arg(
      Arg::with_name("allow-onelevel")
        .long("allow-onelevel")
        .help("do not require slash"),
    )
    .arg(
      Arg::with_name("refname")
        .required(true)
        .help("refname to check"),
    )
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let mut refname = matches.value_of("refname").unwrap().to_string();

  if matches.is_present("normalize") {
    refname = refname.replace("//", "/");
    refname = refname.trim_start_matches("/").to_string();
  }

  let mut ok = util::is_valid_refname(&refname);

  if !refname.contains("/") && !matches.is_present("allow-onelevel") {
    ok = false;
  }

  if !ok {
    std::process::exit(1); // naughty, but this is what git does.
  }

  if matches.is_present("normalize") {
    ctx.println(refname);
  }

  Ok(())
}
