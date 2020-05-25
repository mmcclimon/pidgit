use std::fmt;

use crate::object::GitObject;

pub struct Blob {
  content: Vec<u8>,
}

impl GitObject for Blob {
  fn raw_content(&self) -> Vec<u8> {
    self.content.clone()
  }

  fn type_str(&self) -> &str {
    "blob"
  }
}

impl fmt::Debug for Blob {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    fmt
      .debug_struct("Blob")
      .field("content", &"<raw data>")
      .finish()
  }
}

impl Blob {
  pub fn from_content(content: Vec<u8>) -> Self {
    Self { content }
  }
}
