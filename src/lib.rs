// modules
pub mod cmd;
mod errors;
mod index;
mod lockfile;
mod object;
mod repo;
pub mod util;

#[cfg(test)]
pub mod test_prelude;

// public uses and preludes
pub use crate::lockfile::Lockfile;

// A convenience module appropriate for glob imports
pub mod prelude {
  pub use crate::cmd::Command;
  pub use crate::errors::{PidgitError, Result};
  pub use crate::object::GitObject;
  pub use crate::repo::Repository;
  pub use crate::util;
}

// The actual app implementation. Basic usage:
//   let app = pidgit::new();
//   app.dispatch(&app.get_matches(), std::io::stdout())?;

use crate::cmd::{CommandSet, Context};
use crate::errors::Result;
use crate::repo::Repository;
use clap::{crate_version, App, AppSettings, ArgMatches};

pub struct PidgitApp {
  commands: CommandSet,
}

pub fn new() -> PidgitApp {
  PidgitApp {
    commands: CommandSet::new(),
  }
}

impl PidgitApp {
  // we can't just stick this into a PidgitApp because get_matches() really
  // wants to consume the app.
  fn clap_app(&self) -> App<'static, 'static> {
    App::new("pidgit")
      .version(crate_version!())
      .settings(&[
        AppSettings::SubcommandRequiredElseHelp,
        AppSettings::VersionlessSubcommands,
      ])
      .subcommands(self.commands.apps())
  }

  pub fn get_matches(&self) -> ArgMatches {
    self.clap_app().get_matches()
  }

  pub fn dispatch<W>(
    &self,
    app_matches: &ArgMatches,
    repo: Option<&Repository>,
    writer: W,
  ) -> Result<()>
  where
    W: std::io::Write,
  {
    let cmd_name = app_matches.subcommand_name().expect("no subcommand!");
    let cmd = self.commands.command_named(cmd_name); // might panic
    let matches = app_matches.subcommand_matches(cmd_name).unwrap();

    let ctx = Context::new(repo, writer);

    cmd.run(matches, &ctx)?;

    Ok(())
  }
}
