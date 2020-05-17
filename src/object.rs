use std::path::Path;

// object is a pretty generic name, but hey
#[derive(Debug)]
#[allow(unused)] // for now...
pub enum Object {
  Generic,
  Blob,
  Commit,
  Tag,
  Tree,
  NotFound,
}

impl Object {
  pub fn from_path(path: &Path) -> Self {
    if !path.is_file() {
      return Self::NotFound;
    }

    // TODO
    Self::Generic
  }
}
