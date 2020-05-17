use std::fs::{DirBuilder, File};
use std::path::{Path, PathBuf};

const GITDIR_NAME: &'static str = ".pidgit";

#[derive(Debug)]
pub struct Repository {
  work_tree: PathBuf,
  gitdir:    PathBuf,
}

impl Repository {
  pub fn from_work_tree(dir: &Path) -> Self {
    Repository {
      work_tree: dir.to_path_buf(),
      gitdir:    dir.join(GITDIR_NAME),
    }
  }

  pub fn create_empty(root: &Path) -> Self {
    let gitdir = root.join(GITDIR_NAME);
    DirBuilder::new().create(&gitdir).unwrap();

    Repository {
      work_tree: root.to_path_buf(),
      gitdir:    gitdir,
    }
  }

  pub fn work_tree(&self) -> &PathBuf {
    &self.work_tree
  }

  pub fn gitdir(&self) -> &PathBuf {
    &self.gitdir
  }

  pub fn create_file<P>(&self, path: P) -> Result<File, std::io::Error>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    File::create(self.gitdir.join(path))
  }

  pub fn create_dir<P>(&self, path: P) -> Result<(), std::io::Error>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    DirBuilder::new()
      .recursive(true)
      .create(self.gitdir.join(path))
  }
}
