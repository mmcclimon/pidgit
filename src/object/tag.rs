use crate::object::GitObject;

#[derive(Debug)]
pub struct Tag {
  content: Vec<u8>,
}

impl GitObject for Tag {
  fn raw_content(&self) -> Vec<u8> {
    self.content.clone()
  }

  fn type_str(&self) -> &str {
    "tag"
  }
}

impl Tag {
  pub fn from_content(content: Vec<u8>) -> Self {
    Self { content }
  }
}
