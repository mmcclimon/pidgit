use flate2::read::ZlibDecoder;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::{PidgitError, Result};

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
pub enum Object {
  Blob(RawObject),
  Commit(RawObject),
  Tag(RawObject),
  Tree(RawObject),
}

#[derive(Debug)]
pub struct RawObject {
  pub sha:     String,
  pub size:    u32, // in bytes
  pub content: Vec<u8>,
}

pub trait GitObject {
  fn get_ref(&self) -> &RawObject;

  fn size(&self) -> u32 {
    self.get_ref().size
  }

  // default, should be better
  fn pretty(&self) -> Vec<u8> {
    self.get_ref().content.to_vec()
  }
}

impl Object {
  pub fn from_path(path: &Path) -> Result<Self> {
    let sha = sha_from_path(&path);

    if !path.is_file() {
      return Err(PidgitError::ObjectNotFound(sha));
    }

    let f = File::open(path)?;
    let mut zfile = BufReader::new(ZlibDecoder::new(f));

    let mut buf = vec![];
    zfile.read_until(b'\0', &mut buf)?;

    // ignore null terminator
    let s = std::str::from_utf8(&buf[0..buf.len() - 1])?;
    let bits = s.split(" ").collect::<Vec<_>>();

    let string_type = bits[0];
    let size: u32 = bits[1].parse()?;

    // We could be smarter and not eagerly read objects into memory, but I think
    // this is fine for now.
    let mut content = vec![];
    zfile.read_to_end(&mut content)?;

    let raw = RawObject { sha, size, content };

    let kind = match string_type {
      "commit" => Self::Commit(raw),
      "tag" => Self::Tag(raw),
      "tree" => Self::Tree(raw),
      "blob" => Self::Blob(raw),
      _ => unreachable!(),
    };

    Ok(kind)
  }

  pub fn get_ref(&self) -> &RawObject {
    match self {
      Self::Blob(raw) => raw,
      Self::Commit(raw) => raw,
      Self::Tag(raw) => raw,
      Self::Tree(raw) => raw,
    }
  }

  pub fn size(&self) -> u32 {
    self.get_ref().size
  }

  pub fn string_type(&self) -> &'static str {
    match self {
      Self::Blob(_) => "blob",
      Self::Commit(_) => "commit",
      Self::Tag(_) => "tag",
      Self::Tree(_) => "tree",
    }
  }

  // consume self, turning into a GitObject
  pub fn inflate(self) -> Box<dyn GitObject> {
    match self {
      Self::Blob(raw) => Box::new(Blob::from_raw(raw)),
      Self::Commit(raw) => Box::new(Commit::from_raw(raw)),
      Self::Tag(raw) => Box::new(Tag::from_raw(raw)),
      Self::Tree(raw) => Box::new(Tree::from_raw(raw)),
    }
  }
}

fn sha_from_path(path: &Path) -> String {
  let hunks = path
    .components()
    .map(|c| c.as_os_str().to_string_lossy())
    .collect::<Vec<_>>();

  let l = hunks.len();
  format!("{}{}", hunks[l - 2], hunks[l - 1])
}
