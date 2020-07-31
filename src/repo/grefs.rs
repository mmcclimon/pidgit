use sha1::Sha1;
use std::{
  fs::File,
  io::prelude::*,
  path::{Path, PathBuf},
};

use crate::prelude::*;
use crate::Lockfile;

// This is _so_ silly, but: the word "ref" is already super common in Rust
// code, and I want to avoid ambiguity. Internally, anything that is a git
// ref (like you might find in refs.c) is called a "gref". This means that you
// can say something like `let gref = ...`, which makes some kind of internal
// sense, because `let ref = ...` does not compile.
#[derive(Debug)]
pub struct Grefs {
  git_dir: PathBuf,
}

impl Grefs {
  pub fn new(git_dir: PathBuf) -> Self {
    Grefs { git_dir }
  }

  // give it a path relative to .git_dir, read into a string
  fn read_file<P>(&self, path: P) -> Result<String>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    let mut s = String::new();
    File::open(self.git_dir.join(path))?.read_to_string(&mut s)?;
    Ok(s.trim().to_string())
  }

  // this returns a sha
  pub fn resolve(&self, refstr: &str) -> Result<String> {
    let res = self.read_file(refstr);

    // if we got an error and we're looking for a symref, return a better error.
    if let Err(PidgitError::Io(err)) = res {
      if refstr.starts_with("refs") {
        return Err(PidgitError::RefNotFound(refstr.to_string()));
      } else {
        return Err(PidgitError::Io(err));
      }
    }

    let raw = res.unwrap();

    if raw.starts_with("ref: ") {
      let symref = raw.trim_start_matches("ref: ");
      self.resolve(symref)
    } else {
      Ok(raw)
    }
  }

  pub fn update_head(&self, new_sha: &Sha1) -> Result<()> {
    // we must read the content of .git/HEAD. If that's a gitref, we find the
    // open that other file instead. If it's not a gitref, it must be a sha
    // (i.e., we're in detached head mode), so we lock the head file itself.
    let raw = self.read_file("HEAD")?;

    let ref_path = if raw.starts_with("ref: ") {
      raw.trim_start_matches("ref: ")
    } else {
      // must be a sha
      "HEAD"
    };

    self.update_ref_file(ref_path, &new_sha.hexdigest())
  }

  fn update_ref_file(&self, ref_path: &str, new_sha: &str) -> Result<()> {
    let lockfile = Lockfile::new(self.git_dir.join(ref_path));
    let mut lock = lockfile.lock()?;

    lock.write_all(format!("{}\n", new_sha).as_bytes())?;
    lock.commit()?;

    Ok(())
  }

  pub fn create_branch(&self, refname: &str, sha: &str) -> Result<()> {
    if !util::is_valid_refname(refname) {
      return Err(PidgitError::InvalidRefName(refname.to_string()));
    }

    let pathstr = format!("refs/heads/{}", refname);

    if self.resolve(&pathstr).is_ok() {
      return Err(PidgitError::Generic(format!(
        "A branch named '{}' already exists",
        refname
      )));
    }

    // create parent dir!
    std::fs::create_dir_all(PathBuf::from(&pathstr).parent().unwrap())?;

    self.update_ref_file(&pathstr, sha)
  }
}
