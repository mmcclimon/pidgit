use crate::prelude::*;
use clap::ArgMatches;
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf};

mod add;
mod cat_file;
mod commit;
mod dump_index;
mod dump_tree;
mod hash_object;
mod init;
mod log;
mod ls_files;
mod rev_parse;
mod status;

pub type App = clap::App<'static, 'static>;

pub struct Context<'w> {
  writer: RefCell<Box<dyn std::io::Write + 'w>>,
  repo:   Option<&'w Repository>,
  pwd:    PathBuf,
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
    commands.insert("dump-tree", dump_tree::new());
    commands.insert("hash-object", hash_object::new());
    commands.insert("init", init::new());
    commands.insert("log", log::new());
    commands.insert("ls-files", ls_files::new());
    commands.insert("rev-parse", rev_parse::new());
    commands.insert("status", status::new());

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

  fn run(&self, matches: &ArgMatches, ctx: &Context) -> Result<()>;
}

impl<'w> Context<'w> {
  pub fn new<W>(repo: Option<&'w Repository>, writer: W, pwd: PathBuf) -> Self
  where
    W: std::io::Write + 'w,
  {
    Self {
      repo,
      writer: RefCell::new(Box::new(writer)),
      pwd,
    }
  }

  pub fn repo(&self) -> Result<&Repository> {
    self
      .repo
      .ok_or_else(|| PidgitError::Generic("not a pidgit repository".to_string()))
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
