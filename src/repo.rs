use flate2::{write::ZlibEncoder, Compression};
use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::object::RawObject;
use crate::{PidgitError, Result};

const GITDIR_NAME: &'static str = ".pidgit";

#[derive(Debug)]
pub struct Repository {
  work_tree: PathBuf,
  gitdir:    PathBuf,
}

impl Repository {
  pub fn from_work_tree(dir: &Path) -> Result<Self> {
    if !dir.is_dir() {
      return Err(PidgitError::Generic(format!(
        "cannot instantiate repo from working tree: {} is not a directory",
        dir.display()
      )));
    }

    Ok(Repository {
      work_tree: dir.to_path_buf(),
      gitdir:    dir.join(GITDIR_NAME),
    })
  }

  pub fn from_gitdir(gitdir: &Path) -> Result<Self> {
    let path = gitdir.canonicalize()?;

    let parent = path.parent().ok_or_else(|| {
      PidgitError::Generic(format!(
        "cannot resolve gitdir: {} has no parent",
        path.display()
      ))
    })?;

    Ok(Repository {
      work_tree: parent.to_path_buf(),
      gitdir:    gitdir.to_path_buf(),
    })
  }

  pub fn create_empty(root: &Path) -> Result<Self> {
    let gitdir = root.join(GITDIR_NAME);
    DirBuilder::new().create(&gitdir)?;

    Ok(Repository {
      work_tree: root.to_path_buf(),
      gitdir,
    })
  }

  pub fn work_tree(&self) -> &PathBuf {
    &self.work_tree
  }

  pub fn gitdir(&self) -> &PathBuf {
    &self.gitdir
  }

  pub fn create_file<P>(&self, path: P) -> Result<File>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    File::create(self.gitdir.join(path)).map_err(|e| e.into())
  }

  pub fn create_dir<P>(&self, path: P) -> Result<()>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    DirBuilder::new()
      .recursive(true)
      .create(self.gitdir.join(path))
      .map_err(|e| e.into())
  }

  // give it a path relative to .gitdir, read into a string
  pub fn read_file<P>(&self, path: P) -> Result<String>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    let mut s = String::new();
    File::open(self.gitdir().join(path))?.read_to_string(&mut s)?;
    Ok(s.trim().to_string())
  }

  pub fn object_for_sha(&self, sha: &str) -> Result<RawObject> {
    // make this better, eventually
    if sha.len() != 40 {
      return Err(PidgitError::Generic(format!(
        "malformed sha: {} is not 40 chars",
        sha,
      )));
    }

    if !sha.chars().all(|c| c.is_digit(16)) {
      return Err(PidgitError::Generic(format!(
        "malformed sha: {} contains non-hex characters",
        sha,
      )));
    }

    RawObject::from_path(&self.path_for_sha(sha))
  }

  // NB returns an absolute path!
  pub fn path_for_sha(&self, sha: &str) -> PathBuf {
    self
      .gitdir
      .join(format!("objects/{}/{}", &sha[0..2], &sha[2..]))
  }

  pub fn write_object(&self, obj: &RawObject) -> Result<()> {
    let path = self.path_for_sha(&obj.sha().hexdigest());

    // create parent dir!
    std::fs::create_dir_all(path.parent().unwrap())?;

    let file = File::create(path)?;

    let mut e = ZlibEncoder::new(file, Compression::default());

    e.write_all(&obj.header())?;
    e.write_all(&obj.content())?;
    e.finish()?;

    Ok(())
  }

  fn resolve_ref(&self, refstr: &str) -> Result<RawObject> {
    let raw = self.read_file(refstr)?;

    if raw.starts_with("ref: ") {
      let symref = raw.trim_start_matches("ref: ");
      self.resolve_ref(symref)
    } else {
      self.object_for_sha(&raw)
    }
  }

  pub fn head(&self) -> Result<RawObject> {
    self.resolve_ref("HEAD")
  }

  pub fn resolve_object(&self, name: &str) -> Result<RawObject> {
    match name {
      "head" | "HEAD" | "@" => self.head(),
      _ => Err(PidgitError::ObjectNotFound(name.to_string())),
    }
  }
}
