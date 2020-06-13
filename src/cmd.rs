use crate::prelude::*;
use clap::ArgMatches;

mod add;
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
    add::app(),
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

pub fn dispatch<W>(app_matches: &ArgMatches, mut stdout: &mut W) -> Result<()>
where
  W: std::io::Write,
{
  match app_matches.subcommand() {
    ("add", Some(matches)) => add::run(matches, &mut stdout),
    ("cat-file", Some(matches)) => cat_file::run(matches, &mut stdout),
    ("commit", Some(matches)) => commit::run(matches, &mut stdout),
    ("dump-index", Some(matches)) => dump_index::run(matches, &mut stdout),
    ("hash-object", Some(matches)) => hash_object::run(matches, &mut stdout),
    ("init", Some(matches)) => init::run(matches, &mut stdout),
    ("log", Some(matches)) => log::run(matches, &mut stdout),
    ("ls-files", Some(matches)) => ls_files::run(matches, &mut stdout),
    ("rev-parse", Some(matches)) => rev_parse::run(matches, &mut stdout),
    _ => unreachable!("unknown command!"),
  }
}
