use clap::{App, Arg, ArgMatches};

use crate::prelude::*;

pub fn command() -> Command {
  (app, run)
}

// this is, for now, a weak imitation of git.
fn app() -> ClapApp {
  App::new("diff-tree")
    .about("compares the content and mode of blobs found via two tree objects")
    .arg(
      Arg::with_name("tree1")
        .takes_value(true)
        .required(true)
        .help("first tree"),
    )
    .arg(
      Arg::with_name("tree2")
        .takes_value(true)
        .required(true)
        .help("second tree"),
    )
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;

  let tree1 = repo.resolve_object(matches.value_of("tree1").unwrap())?;
  let tree2 = repo.resolve_object(matches.value_of("tree2").unwrap())?;

  let tree1 = tree1.as_tree()?;
  let tree2 = tree2.as_tree()?;

  println!("{:?}", tree1);
  println!("{:?}", tree2);

  Ok(())
}
