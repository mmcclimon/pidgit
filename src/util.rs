use sha1::Sha1;
use std::path::{Path, PathBuf};

use crate::prelude::*;

pub fn find_repo() -> Option<Repository> {
  // so that PIDGIT_DIR=.git works for quick desk-checking
  if let Ok(dir) = std::env::var("PIDGIT_DIR") {
    let path = PathBuf::from(dir)
      .canonicalize()
      .expect("couldn't canonicalize PIDGIT_DIR");
    return Repository::from_git_dir(&path).ok();
  }

  let pwd = std::env::current_dir();

  if pwd.is_err() {
    return None;
  }

  let repo = pwd
    .unwrap()
    .ancestors()
    .filter(|p| p.join(".pidgit").is_dir())
    .next()
    .map(|p| Repository::from_work_tree(p).unwrap());

  repo
}

/// Given a path to an object (like .git/objects/04/2348ac8d3), this extracts
/// the 40-char sha and returns it as a string.
pub fn sha_from_path(path: &Path) -> String {
  let hunks = path
    .components()
    .map(|c| c.as_os_str().to_str().unwrap())
    .collect::<Vec<_>>();

  let l = hunks.len();
  format!("{}{}", hunks[l - 2], hunks[l - 1])
}

// Get the sha for a file on disk, without reading the whole thing into memory.
pub fn compute_sha_for_path(path: &Path) -> Result<Sha1> {
  use std::fs::File;
  use std::io::{BufRead, BufReader};

  let mut reader = BufReader::new(File::open(&path)?);
  let mut sha = Sha1::new();
  let meta = path.metadata()?;

  sha.update(format!("blob {}\0", meta.len()).as_bytes());

  loop {
    let buf = reader.fill_buf()?;
    let len = buf.len();

    // EOF
    if len == 0 {
      break;
    }

    sha.update(&buf);
    reader.consume(len);
  }

  Ok(sha)
}
