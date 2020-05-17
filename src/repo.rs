use std::fs::{DirBuilder, File};
use std::path::{Path, PathBuf};

use crate::object::Object;
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
      gitdir:    gitdir,
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

  pub fn object_for_sha(&self, sha: &str) -> Result<Object> {
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

    let path = self
      .gitdir
      .join(format!("objects/{}/{}", &sha[0..2], &sha[2..]));

    let obj = Object::from_path(&path);

    Ok(obj)
  }
}
