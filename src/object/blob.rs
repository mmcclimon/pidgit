use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use crate::prelude::*;

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

  pub fn from_path(path: &PathBuf) -> Result<Self> {
    let mut content = vec![];
    let mut reader = BufReader::new(File::open(&path)?);
    reader.read_to_end(&mut content)?;
    Ok(Self::from_content(content))
  }
}
