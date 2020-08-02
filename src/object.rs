use flate2::read::ZlibDecoder;
use sha1::Sha1;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::prelude::*;

mod blob;
mod commit;
mod tag;
mod tree;

pub use blob::Blob;
pub use commit::Commit;
pub use commit::Person;
pub use tag::Tag;
pub use tree::{PathEntry, Tree, TreeItem};

// object is a pretty generic name, but hey
#[derive(Debug)]
#[allow(unused)]
pub enum Object {
  Blob(Blob),
  Commit(Commit),
  Tree(Tree),
  Tag(Tag),
}

pub trait GitObject: std::fmt::Debug {
  // raw bytes, no header
  fn raw_content(&self) -> Vec<u8>;

  fn type_str(&self) -> &str;

  fn size(&self) -> usize {
    self.raw_content().len()
  }

  // returns bytes, because mostly it's useful for generating the sha
  fn header(&self) -> Vec<u8> {
    format!("{} {}\0", self.type_str(), self.size())
      .as_bytes()
      .to_vec()
  }

  fn sha(&self) -> Sha1 {
    let mut sha = Sha1::new();
    sha.update(&self.header());
    sha.update(&self.raw_content());
    sha
  }

  // default
  fn pretty(&self) -> Vec<u8> {
    self.raw_content().clone()
  }
}

impl Object {
  pub fn from_git_db(path: &Path) -> Result<Self> {
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

    // We could be smarter and not eagerly read objects into memory, but I think
    // this is fine for now.
    let mut content = vec![];
    zfile.read_to_end(&mut content)?;

    let ret = match string_type {
      "blob" => Object::Blob(Blob::from_content(content)),
      "commit" => Object::Commit(Commit::from_content(content)),
      "tag" => Object::Tag(Tag::from_content(content)),
      "tree" => Object::Tree(Tree::from_content(content)),
      _ => panic!("unknown object type {}", s),
    };

    Ok(ret)
  }

  // consume self, turning into a GitObject
  pub fn into_inner(self) -> Box<dyn GitObject> {
    match self {
      Object::Blob(blob) => Box::new(blob),
      Object::Commit(commit) => Box::new(commit),
      Object::Tag(tag) => Box::new(tag),
      Object::Tree(tree) => Box::new(tree),
    }
  }

  pub fn get_ref(&self) -> &dyn GitObject {
    match self {
      Object::Blob(blob) => blob,
      Object::Commit(commit) => commit,
      Object::Tag(tag) => tag,
      Object::Tree(tree) => tree,
    }
  }

  pub fn as_blob(self) -> Result<Blob> {
    match self {
      Object::Blob(blob) => Ok(blob),
      _ => Err(PidgitError::InvalidObject("blob")),
    }
  }

  pub fn as_commit(self) -> Result<Commit> {
    match self {
      Object::Commit(commit) => Ok(commit),
      _ => Err(PidgitError::InvalidObject("commit")),
    }
  }

  pub fn as_tag(self) -> Result<Tag> {
    match self {
      Object::Tag(tag) => Ok(tag),
      _ => Err(PidgitError::InvalidObject("tag")),
    }
  }

  pub fn as_tree(self) -> Result<Tree> {
    match self {
      Object::Tree(tree) => Ok(tree),
      _ => Err(PidgitError::InvalidObject("tree")),
    }
  }

  pub fn sha(&self) -> Sha1 {
    self.get_ref().sha()
  }
}
