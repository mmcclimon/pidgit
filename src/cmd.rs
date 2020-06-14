use crate::prelude::*;
use clap::ArgMatches;
use std::{cell::RefCell, collections::BTreeMap};

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

pub struct Stdout<'w> {
  writer: RefCell<Box<dyn std::io::Write + 'w>>,
}

pub struct CommandSet {
  commands: BTreeMap<&'static str, Box<dyn Command>>,
}

impl CommandSet {
  pub fn new() -> Self {
    let mut commands = BTreeMap::new();

    commands.insert("add", add::new());
    commands.insert("cat-file", cat_file::new());
    commands.insert("commit", commit::new());
    commands.insert("dump-index", dump_index::new());
    commands.insert("hash-object", hash_object::new());
    commands.insert("init", init::new());
    commands.insert("log", log::new());
    commands.insert("ls-files", ls_files::new());
    commands.insert("rev-parse", rev_parse::new());

    Self { commands }
  }

  pub fn apps(&self) -> impl IntoIterator<Item = App> + '_ {
    self.commands.values().map(|cmd| cmd.app())
  }

  pub fn command_named<'a>(&'a self, name: &'a str) -> &'a Box<dyn Command> {
    self.commands.get(&name).expect("command not found!")
  }
}

pub trait Command: std::fmt::Debug {
  fn app(&self) -> App;

  fn run(&self, matches: &ArgMatches, stdout: &Stdout) -> Result<()>;
}

impl<'w> Stdout<'w> {
  pub fn new<W: std::io::Write + 'w>(writer: W) -> Self {
    Self {
      writer: RefCell::new(Box::new(writer)),
    }
  }

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
