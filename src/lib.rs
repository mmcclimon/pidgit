pub mod cmd;
mod errors;
mod index;
mod object;
mod repo;
mod util;

pub use errors::{PidgitError, Result};
pub use object::Object;
pub use repo::Repository;

pub fn find_repo() -> Result<Repository> {
  // so that PIDGIT_DIR=.git works for quick desk-checking
  if let Ok(dir) = std::env::var("PIDGIT_DIR") {
    use std::path::PathBuf;
    let path = PathBuf::from(dir).canonicalize()?;

    return Repository::from_gitdir(&path);
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
