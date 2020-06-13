pub mod cmd;
mod errors;
mod index;
mod lockfile;
mod object;
mod repo;
pub mod util;

pub use crate::lockfile::Lockfile;

use clap::{crate_version, App, AppSettings};

pub fn app() -> App<'static, 'static> {
  App::new("pidgit")
    .version(crate_version!())
    .settings(&[
      AppSettings::SubcommandRequiredElseHelp,
      AppSettings::VersionlessSubcommands,
    ])
    .subcommands(cmd::command_apps())
}

// A convenience module appropriate for glob imports
pub mod prelude {
  pub use crate::errors::{PidgitError, Result};
  pub use crate::object::GitObject;
  pub use crate::repo::Repository;
  pub use crate::util;
}

#[cfg(test)]
pub mod test_prelude {
  pub use super::errors::{PidgitError, Result};
  pub use assert_fs::prelude::*;
  pub use assert_fs::TempDir;
  pub use predicates::prelude::*;
  pub use serial_test::serial;

  use std::io::Cursor;

  pub fn tempdir() -> TempDir {
    let d = TempDir::new().expect("couldn't make tempdir");
    assert!(d.path().is_dir());
    d
  }

  pub fn run_pidgit(args: Vec<&str>) -> Result<String> {
    // Assume default environment. If later we want to test this, the guts of
    // this function can move into run_pidgit_raw, and tests can call that if
    // they don't want the munging.
    std::env::remove_var("PIDGIT_DIR");

    let mut stdout = Cursor::new(vec![]);
    let full_args = std::iter::once("pidgit").chain(args);
    let matches = super::app().get_matches_from_safe(full_args)?;
    super::cmd::dispatch(&matches, &mut stdout)?;
    Ok(String::from_utf8(stdout.into_inner())?)
  }
}
