use crate::object::{GitObject, RawObject};

#[derive(Debug)]
pub struct Blob {
  raw: RawObject,
}

impl GitObject for Blob {
  fn get_ref(&self) -> &RawObject {
    &self.raw
  }
}

impl Blob {
  pub fn from_raw(raw: RawObject) -> Self {
    Self { raw }
  }
}
