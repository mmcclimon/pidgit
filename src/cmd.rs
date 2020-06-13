use crate::prelude::*;
use clap::ArgMatches;
use std::cell::RefCell;

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

pub trait Writeable = std::io::Write + std::fmt::Debug;

#[derive(Debug)]
pub struct Stdout<W: Writeable> {
  writer: RefCell<W>,
}

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

pub fn dispatch<'w, W: Writeable>(
  app_matches: &ArgMatches,
  writer: &'w mut W,
) -> Result<()> {
  let stdout = Stdout {
    writer: RefCell::new(writer),
  };

  let cmd_name = app_matches.subcommand_name().expect("no subcommand!");

  let mut cmd = match cmd_name {
    "add" => add::new(stdout),
    "cat-file" => cat_file::new(stdout),
    "commit" => commit::new(stdout),
    "dump-index" => dump_index::new(stdout),
    "hash-object" => hash_object::new(stdout),
    "init" => init::new(stdout),
    "log" => log::new(stdout),
    "ls-files" => ls_files::new(stdout),
    "rev-parse" => rev_parse::new(stdout),
    _ => unreachable!("unknown command!"),
  };

  cmd.run(app_matches.subcommand_matches(cmd_name).unwrap())
}

pub trait Command<W: Writeable>: std::fmt::Debug {
  fn stdout(&self) -> &Stdout<W>;

  fn run(&mut self, matches: &ArgMatches) -> Result<()>;

  fn println(&self, out: String) {
    self.stdout().println(out);
  }
}

impl<W: Writeable> Stdout<W> {
  pub fn println(&self, out: String) {
    writeln!(self.writer.borrow_mut(), "{}", out).unwrap();
  }

  pub fn println_raw(&self, out: &[u8]) -> Result<()> {
    let mut writer = self.writer.borrow_mut();
    writer.write_all(out)?;
    writer.write(b"\n")?;
    Ok(())
  }
}
