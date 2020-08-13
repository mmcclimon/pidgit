use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

use crate::object::TreeItem;
use crate::prelude::*;

pub fn command() -> Command {
  (app, run)
}

fn app() -> ClapApp {
  App::new("dump-tree")
    .about("dump a tree (for debugging)")
    .arg(
      Arg::with_name("commit") // really, should implement tree-ish
        .default_value("HEAD")
        .help("commit to dump"),
    )
}

fn run(matches: &ArgMatches, ctx: &Context) -> Result<()> {
  let repo = ctx.repo()?;

  let to_find = matches.value_of("commit").unwrap();
  let head = repo.resolve_object(to_find)?;

  let commit = head.as_commit()?;

  let tree = repo
    .resolve_object(&commit.tree().hexdigest())?
    .into_inner();

  print_tree(&repo, &tree.sha(), &PathBuf::from(""))?;

  Ok(())
}

fn print_tree(repo: &Repository, sha: &Sha, prefix: &PathBuf) -> Result<()> {
  let tree = repo.resolve_object(&sha.hexdigest())?.as_tree()?;

  for (path, entry) in tree.entries() {
    if let TreeItem::Entry(e) = entry {
      if e.is_tree() {
        print_tree(repo, e.sha(), &prefix.join(path))?;
      } else {
        println!(
          "{} {} {}",
          e.mode().long(),
          e.sha(),
          prefix.join(path).display()
        )
      }
    }
  }

  Ok(())
}
