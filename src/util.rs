use crate::prelude::*;
use std::path::{Path, PathBuf};

pub fn find_repo() -> Result<Repository> {
  // so that PIDGIT_DIR=.git works for quick desk-checking
  if let Ok(dir) = std::env::var("PIDGIT_DIR") {
    let path = PathBuf::from(dir).canonicalize()?;
    return Repository::from_git_dir(&path);
  }

  let pwd = std::env::current_dir()?;

  let repo = pwd
    .ancestors()
    .filter(|p| p.join(".pidgit").is_dir())
    .next()
    .map_or_else(
      || Err(PidgitError::Generic("not a pidgit repository".to_string())),
      |p| Repository::from_work_tree(p),
    );

  repo
}

/// Given a path to an object (like .git/objects/04/2348ac8d3), this extracts
/// the 40-char sha and returns it as a string.
pub fn sha_from_path(path: &Path) -> String {
  let hunks = path
    .components()
    .map(|c| c.as_os_str().to_string_lossy())
    .collect::<Vec<_>>();

  let l = hunks.len();
  format!("{}{}", hunks[l - 2], hunks[l - 1])
}
