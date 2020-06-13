pub mod cmd;
mod errors;
mod index;
mod lockfile;
mod object;
mod repo;
pub mod util;

pub use crate::lockfile::Lockfile;

// A convenience module appropriate for glob imports
pub mod prelude {
  pub use crate::errors::{PidgitError, Result};
  pub use crate::object::GitObject;
  pub use crate::repo::Repository;
  pub use crate::util;
}

#[cfg(test)]
pub mod test_prelude {
  pub use assert_fs::prelude::*;
  pub use assert_fs::TempDir;
  pub use serial_test::serial;

  pub fn tempdir() -> TempDir {
    let d = TempDir::new().expect("couldn't make tempdir");
    assert!(d.path().is_dir());
    d
  }
}
