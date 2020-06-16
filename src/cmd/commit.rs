use chrono::Local;
use clap::{App, Arg, ArgMatches};

use crate::cmd::Context;
use crate::object::Person;
use crate::prelude::*;

#[derive(Debug)]
struct CommitCmd;

pub fn new() -> Box<dyn Command> {
  Box::new(CommitCmd {})
}

impl Command for CommitCmd {
  fn app(&self) -> App<'static, 'static> {
    App::new("commit")
      .about("record changes to the repository")
      .arg(
        Arg::with_name("message")
          .short("m")
          .long("message")
          .takes_value(true)
          .value_name("msg")
          .required(true)
          .help("use this as the message"),
      )
  }

  fn run(&self, matches: &ArgMatches, ctx: &Context) -> Result<()> {
    let repo = ctx.repo()?;

    let now = Local::now();
    let fixed = now.with_timezone(now.offset());

    // first pass, will improve later
    let who = Person {
      name:  std::env::var("GIT_AUTHOR_NAME").unwrap(),
      email: std::env::var("GIT_AUTHOR_EMAIL").unwrap(),
      date:  fixed,
    };

    let msg = matches.value_of("message").unwrap().to_string();

    let commit = repo.commit(&msg, who.clone(), who)?;

    ctx.println(format!(
      "[{}] {}",
      &commit.sha().hexdigest()[0..8],
      commit.title()
    ));

    Ok(())
  }
}
