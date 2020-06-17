use clap::{App, ArgMatches};

use crate::cmd::Context;
use crate::prelude::*;

#[derive(Debug)]
struct Status;

pub fn new() -> Box<dyn Command> {
  Box::new(Status {})
}

impl Command for Status {
  fn app(&self) -> App<'static, 'static> {
    // this doesn't have all the smarts git does, for now
    App::new("status").about("show the working tree status")
  }

  fn run(&self, _matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;

    let index = repo.index()?;

    for f in repo
      .workspace()
      .list_files()?
      .iter()
      .filter(|p| !index.is_tracked(&p.to_string_lossy()))
    {
      ctx.println(format!("?? {}", f.display()));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::test_prelude::*;

  #[test]
  fn only_untracked_files() {
    let tr = new_empty_repo();
    tr.write_file("file.txt", "file content");
    tr.write_file("other.txt", "more content");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert!(stdout.contains("?? file.txt"));
    assert!(stdout.contains("?? other.txt"));
  }

  #[test]
  fn untracked_and_others() {
    let tr = new_empty_repo();
    tr.write_file("committed.txt", "to be committed");

    #[rustfmt::skip]
    tr.run_pidgit(vec!["add", "committed.txt"]).expect("bad add");
    tr.commit("a commit message").expect("could not commit");

    tr.write_file("file.txt", "uncommitted");

    let stdout = tr.run_pidgit(vec!["status"]).unwrap();
    assert!(stdout.contains("?? file.txt"));
    assert!(!stdout.contains("?? committed.txt"));
  }
}
