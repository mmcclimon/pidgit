use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Repository {
  work_tree: PathBuf,
  gitdir:    PathBuf,
}

impl Repository {
  pub fn from_work_tree(dir: &Path) -> Self {
    Repository {
      work_tree: dir.to_path_buf(),
      gitdir:    dir.join(".pidgit"),
    }
  }

  pub fn work_tree(&self) -> &PathBuf {
    &self.work_tree
  }
}
