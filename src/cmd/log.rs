use clap::{App, Arg, ArgMatches};

use crate::cmd::Context;
use crate::object::Object;
use crate::prelude::*;

#[derive(Debug)]
struct Log;

pub fn new() -> Box<dyn Command> {
  Box::new(Log {})
}

impl Command for Log {
  fn app(&self) -> App<'static, 'static> {
    App::new("log").about("show commit logs").arg(
      Arg::with_name("ref")
        .default_value("HEAD")
        .help("ref to start"),
    )
  }

  fn run(&self, matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;

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
      ctx.println(format!("{} {}", &c.sha().hexdigest()[0..8], c.title()));
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
