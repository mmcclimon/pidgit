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
    .arg(Arg::with_name("branch-name").help("new branch name"))
    .arg(Arg::with_name("start-point").help("starting point of the branch"))
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;

  if let Some(name) = matches.value_of("branch-name") {
    let start_ref = matches.value_of("start-point").unwrap_or("HEAD");
    let start = repo.resolve_object(start_ref)?;
    repo.grefs().create_branch(name, &start.sha().hexdigest())?;
    return Ok(());
  }

  ctx.println("branch".to_string());
  Ok(())
}
