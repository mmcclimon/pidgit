use clap::{App, Arg, ArgMatches};

use crate::prelude::*;

pub fn command() -> Command {
  (app, run)
}

fn app() -> ClapApp {
  App::new("branch")
    .about("list, create, or delete branches")
    .arg(
      Arg::with_name("verbose")
        .long("verbose")
        .short("v")
        .help("print sha and commit message of branch tip"),
    )
    .arg(Arg::with_name("newbranch").help("new branch name"))
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;

  if let Some(name) = matches.value_of("newbranch") {
    let head = repo.head().expect("no head?");
    repo.grefs().create_branch(name, &head.sha().hexdigest())?;
    return Ok(());
  }

  ctx.println("branch".to_string());
  Ok(())
}
