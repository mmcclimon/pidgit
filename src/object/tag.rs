use crate::object::{GitObject, RawObject};

#[derive(Debug)]
pub struct Tag {
  raw: RawObject,
}

impl GitObject for Tag {
  fn get_ref(&self) -> &RawObject {
    &self.raw
  }
}

impl Tag {
  pub fn from_raw(raw: RawObject) -> Self {
    Self { raw }
  }
}
