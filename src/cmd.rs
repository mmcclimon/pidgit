use crate::Result;
use clap::ArgMatches;

mod cat_file;
mod hash_object;
mod init;
mod log;
mod rev_parse;

pub type App = clap::App<'static, 'static>;

pub fn command_apps() -> impl IntoIterator<Item = App> {
  vec![
    cat_file::app(),
    hash_object::app(),
    init::app(),
    log::app(),
    rev_parse::app(),
  ]
}

pub fn dispatch(app_matches: &ArgMatches) -> Result<()> {
  match app_matches.subcommand() {
    ("cat-file", Some(matches)) => cat_file::run(matches),
    ("hash-object", Some(matches)) => hash_object::run(matches),
    ("init", Some(matches)) => init::run(matches),
    ("log", Some(matches)) => log::run(matches),
    ("rev-parse", Some(matches)) => rev_parse::run(matches),
    _ => unreachable!("unknown command!"),
  }
}
