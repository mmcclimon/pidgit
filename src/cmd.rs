use crate::prelude::*;
use clap::ArgMatches;

mod cat_file;
mod commit;
mod dump_index;
mod hash_object;
mod init;
mod log;
mod ls_files;
mod rev_parse;

pub type App = clap::App<'static, 'static>;

pub fn command_apps() -> impl IntoIterator<Item = App> {
  vec![
    cat_file::app(),
    commit::app(),
    dump_index::app(),
    hash_object::app(),
    init::app(),
    log::app(),
    ls_files::app(),
    rev_parse::app(),
  ]
}

pub fn dispatch(app_matches: &ArgMatches) -> Result<()> {
  match app_matches.subcommand() {
    ("cat-file", Some(matches)) => cat_file::run(matches),
    ("commit", Some(matches)) => commit::run(matches),
    ("dump-index", Some(matches)) => dump_index::run(matches),
    ("hash-object", Some(matches)) => hash_object::run(matches),
    ("init", Some(matches)) => init::run(matches),
    ("log", Some(matches)) => log::run(matches),
    ("ls-files", Some(matches)) => ls_files::run(matches),
    ("rev-parse", Some(matches)) => rev_parse::run(matches),
    _ => unreachable!("unknown command!"),
  }
}
