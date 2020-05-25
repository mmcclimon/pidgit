pub mod cmd;
mod errors;
mod index;
mod object;
mod repo;
pub mod util;

// pub use errors::{PidgitError, Result};
// pub use object::Object;
// pub use repo::Repository;

// A convenience module appropriate for glob imports (`use chrono::prelude::*;`).
pub mod prelude {
  pub use crate::errors::{PidgitError, Result};
  pub use crate::object::GitObject;
  pub use crate::repo::Repository;
  pub use crate::util;
}
