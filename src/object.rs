use flate2::read::ZlibDecoder;
use sha1::Sha1;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::{util, PidgitError, Result};

mod blob;
mod commit;
mod tag;
mod tree;

pub use blob::Blob;
pub use commit::Commit;
pub use tag::Tag;
pub use tree::Tree;

// object is a pretty generic name, but hey
#[derive(Debug)]
#[allow(unused)]
pub enum Object {
  Blob,
  Commit,
  Tree,
  Tag,
}

impl Object {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Blob => "blob",
      Self::Commit => "commit",
      Self::Tag => "tag",
      Self::Tree => "tree",
    }
  }

  pub fn from_str(s: &str) -> Self {
    match s {
      "blob" => Self::Blob,
      "commit" => Self::Commit,
      "tag" => Self::Tag,
      "tree" => Self::Tree,
      _ => panic!("unknown object type {}", s),
    }
  }
}

pub struct RawObject {
  kind:    Object,
  size:    u32, // in bytes
  content: Vec<u8>,
  header:  Vec<u8>,
}

impl fmt::Debug for RawObject {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    fmt
      .debug_struct("RawObject")
      .field("kind", &self.kind)
      .field("size", &self.size)
      .field("content", &"<raw data>")
      .finish()
  }
}

pub trait GitObject: std::fmt::Debug {
  fn get_ref(&self) -> &RawObject;

  fn size(&self) -> u32 {
    self.get_ref().size
  }

  // default, should be better
  fn pretty(&self) -> Vec<u8> {
    self.get_ref().content.to_vec()
  }
}

impl RawObject {
  pub fn from_path(path: &Path) -> Result<Self> {
    let sha = util::sha_from_path(&path);

    if !path.is_file() {
      return Err(PidgitError::ObjectNotFound(sha));
    }

    let f = File::open(path)?;
    let mut zfile = BufReader::new(ZlibDecoder::new(f));

    let mut buf = vec![];
    zfile.read_until(b'\0', &mut buf)?;
    buf.pop(); // ignore null terminator

    let s = std::str::from_utf8(&buf)?;
    let bits = s.split(" ").collect::<Vec<_>>();

    let string_type = bits[0];
    let size: u32 = bits[1].parse()?;

    // We could be smarter and not eagerly read objects into memory, but I think
    // this is fine for now.
    let mut content = vec![];
    zfile.read_to_end(&mut content)?;

    let kind = Object::from_str(string_type);
    let header = util::header_for(&kind, &content);

    Ok(RawObject {
      kind,
      size,
      content,
      header,
    })
  }

  pub fn size(&self) -> u32 {
    self.size
  }

  pub fn kind(&self) -> &Object {
    &self.kind
  }

  pub fn content(&self) -> &[u8] {
    &self.content
  }

  pub fn sha(&self) -> Sha1 {
    let mut sha = Sha1::new();
    sha.update(&self.header());
    sha.update(&self.content);
    sha
  }

  pub fn header(&self) -> &[u8] {
    &self.header
  }

  // consume self, turning into a GitObject
  pub fn inflate(self) -> Box<dyn GitObject> {
    match self.kind {
      Object::Blob => Box::new(Blob::from_raw(self)),
      Object::Commit => Box::new(Commit::from_raw(self)),
      Object::Tag => Box::new(Tag::from_raw(self)),
      Object::Tree => Box::new(Tree::from_raw(self)),
    }
  }

  // dunno about this, but ok
  pub fn from_content(kind: Object, content: Vec<u8>) -> Result<Self> {
    let header = util::header_for(&kind, &content);

    Ok(RawObject {
      kind,
      size: content.len() as u32,
      content,
      header,
    })
  }
}
