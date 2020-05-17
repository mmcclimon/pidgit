pub mod cmd;
pub mod repo;

pub use repo::Repository;

pub fn find_repo() -> Option<Repository> {
  let pwd = std::env::current_dir().unwrap();

  let repo = pwd
    .ancestors()
    .filter(|p| p.join(".pidgit").is_dir())
    .next()
    .map(|p| Repository::from_work_tree(p));

  repo
}
