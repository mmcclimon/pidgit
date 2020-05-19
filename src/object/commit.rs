use crate::object::{GitObject, RawObject};

#[derive(Debug)]
pub struct Commit {
  raw: RawObject,
}

impl GitObject for Commit {
  fn get_ref(&self) -> &RawObject {
    &self.raw
  }
}

impl Commit {
  pub fn from_raw(raw: RawObject) -> Self {
    Self { raw }
  }
}
