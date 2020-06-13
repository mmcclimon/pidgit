use clap::{App, Arg, ArgMatches};

use crate::cmd::{Stdout, Writeable};
use crate::object::Object;
use crate::prelude::*;

#[derive(Debug)]
struct Log<W: Writeable> {
  stdout: Stdout<W>,
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("log").about("show commit logs").arg(
    Arg::with_name("ref")
      .default_value("HEAD")
      .help("ref to start"),
  )
}

pub fn new<'w, W: 'w + Writeable>(stdout: Stdout<W>) -> Box<dyn Command<W> + 'w> {
  Box::new(Log { stdout })
}

impl<W: Writeable> Command<W> for Log<W> {
  fn stdout(&self) -> &Stdout<W> {
    &self.stdout
  }

  fn run(&mut self, matches: &ArgMatches) -> Result<()> {
    let repo = util::find_repo()?;

    let to_find = matches.value_of("ref").unwrap();
    let object = repo.resolve_object(to_find)?;

    let mut c = match object {
      Object::Commit(commit) => commit,
      _ => {
        return Err(PidgitError::Generic(format!(
          "ref {} is not a commit!",
          to_find,
        )))
      },
    };

    // this is terrible, but expedient
    loop {
      self.println(format!("{} {}", &c.sha().hexdigest()[0..8], c.title()));
      let mut parents = c.parents(&repo);
      if let Some(parent) = parents.pop() {
        c = parent;
      } else {
        break;
      }
    }

    Ok(())
  }
}
