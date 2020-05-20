use crate::Result;
use clap::ArgMatches;

mod cat_file;
mod hash_object;
mod init;

pub type App = clap::App<'static, 'static>;

pub fn command_apps() -> impl IntoIterator<Item = App> {
  vec![init::app(), cat_file::app(), hash_object::app()]
}

pub fn dispatch(app_matches: &ArgMatches) -> Result<()> {
  match app_matches.subcommand() {
    ("init", Some(matches)) => init::run(matches),
    ("cat-file", Some(matches)) => cat_file::run(matches),
    ("hash-object", Some(matches)) => hash_object::run(matches),
    _ => unreachable!("unknown command!"),
  }
}
