use clap::{App, ArgMatches};
use std::path::PathBuf;

use crate::cmd::Context;
use crate::object::TreeItem;
use crate::prelude::*;

#[derive(Debug)]
struct DumpTree;

pub fn new() -> Box<dyn Command> {
  Box::new(DumpTree {})
}

impl Command for DumpTree {
  fn app(&self) -> App<'static, 'static> {
    App::new("dump-tree").about("dump a tree (for debugging)")
  }

  fn run(&self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;
    let head = repo.resolve_object("head")?;

    let commit = head.as_commit()?;

    let tree = repo.resolve_object(commit.tree())?.into_inner();

    print_tree(&repo, &tree.sha().hexdigest(), &PathBuf::from(""))?;

    Ok(())
  }
}

fn print_tree(repo: &Repository, sha: &str, prefix: &PathBuf) -> Result<()> {
  let tree = repo.resolve_object(sha)?.as_tree()?;

  for (path, entry) in tree.entries() {
    if let TreeItem::Entry(e) = entry {
      if e.is_tree() {
        print_tree(repo, e.sha(), &path)?;
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
