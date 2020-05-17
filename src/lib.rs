pub mod cmd;
mod errors;
mod repo;

pub use errors::{PidgitError, Result};
pub use repo::Repository;

pub fn find_repo() -> Result<Repository> {
  let pwd = std::env::current_dir()?;

  let repo = pwd
    .ancestors()
    .filter(|p| p.join(".pidgit").is_dir())
    .next()
    .map(|p| Repository::from_work_tree(p))
    .ok_or_else(|| PidgitError::Generic("not a pidgit repository".to_string()));

  repo
}
