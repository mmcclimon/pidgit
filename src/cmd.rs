use crate::prelude::*;
use clap::ArgMatches;
use std::{
  cell::RefCell, collections::BTreeMap, io::prelude::*, path::PathBuf,
  process::Child,
};

mod add;
mod cat_file;
mod commit;
mod diff;
mod dump_index;
mod dump_tree;
mod hash_object;
mod init;
mod log;
mod ls_files;
mod rev_parse;
mod status;

pub type ClapApp = clap::App<'static, 'static>;

pub type Command = (CommandApp, CommandRunner);
pub type CommandApp = fn() -> ClapApp;
pub type CommandRunner = fn(&ArgMatches, &Context) -> Result<()>;

pub struct Context<'w> {
  writer: RefCell<Box<dyn std::io::Write + 'w>>,
  repo:   Option<&'w Repository>,
  pwd:    PathBuf,
  pager:  RefCell<Option<Child>>,
}

pub struct CommandSet {
  commands: BTreeMap<&'static str, Command>,
}

impl CommandSet {
  pub fn new() -> Self {
    let mut commands = BTreeMap::new();

    commands.insert("add", add::command());
    commands.insert("cat-file", cat_file::command());
    commands.insert("commit", commit::command());
    commands.insert("diff", diff::command());
    commands.insert("dump-index", dump_index::command());
    commands.insert("dump-tree", dump_tree::command());
    commands.insert("hash-object", hash_object::command());
    commands.insert("init", init::command());
    commands.insert("log", log::command());
    commands.insert("ls-files", ls_files::command());
    commands.insert("rev-parse", rev_parse::command());
    commands.insert("status", status::command());

    Self { commands }
  }

  pub fn apps(&self) -> impl IntoIterator<Item = ClapApp> + '_ {
    self.commands.values().map(|cmd| cmd.0())
  }

  pub fn command_named<'a>(&'a mut self, name: &'a str) -> CommandRunner {
    self.commands.get(name).expect("command not found!").1
  }
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
      pager: RefCell::new(None),
    }
  }

  pub fn repo(&self) -> Result<&Repository> {
    self
      .repo
      .ok_or_else(|| PidgitError::Generic("not a pidgit repository".to_string()))
  }

  pub fn println(&self, out: String) {
    if self.pager.borrow().is_some() {
      writeln!(
        self
          .pager
          .borrow_mut()
          .as_mut()
          .unwrap()
          .stdin
          .as_mut()
          .unwrap(),
        "{}",
        out
      )
      .unwrap();
      return;
    }

    writeln!(self.writer.borrow_mut(), "{}", out).unwrap();
  }

  pub fn println_color(&self, out: String, style: ansi_term::Style) {
    if self.pager.borrow().is_some() {
      writeln!(
        self
          .pager
          .borrow_mut()
          .as_mut()
          .unwrap()
          .stdin
          .as_mut()
          .unwrap(),
        "{}",
        util::colored(&out, style),
      )
      .unwrap();
      return;
    }

    writeln!(self.writer.borrow_mut(), "{}", util::colored(&out, style)).unwrap();
  }

  pub fn println_raw(&self, out: &[u8]) -> Result<()> {
    let mut writer = self.writer.borrow_mut();
    writer.write_all(out)?;
    writer.write(b"\n")?;
    Ok(())
  }

  pub fn setup_pager(&self) -> Result<()> {
    if self.pager.borrow().is_some() || !atty::is(atty::Stream::Stdout) {
      return Ok(());
    }

    self.pager.replace(Some(new_pager()));

    Ok(())
  }

  pub fn maybe_wait_for_pager(&self) {
    if self.pager.borrow().is_none() {
      return;
    }

    self
      .pager
      .borrow_mut()
      .as_mut()
      .unwrap()
      .wait()
      .expect("couldn't waitpid?");
  }
}

fn new_pager() -> Child {
  use std::process::{Command, Stdio};
  // TODO: allow customization of this.
  let process = Command::new("less")
    .args(&["-R"])
    .stdin(Stdio::piped())
    .spawn()
    .expect("could not open pager!");

  process
}
