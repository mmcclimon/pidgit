use flate2::read::ZlibDecoder;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::{PidgitError, Result};

// object is a pretty generic name, but hey
// TODO: storing these as strings is naive, because blobs can contain arbitrary
// data. Also, all of these object types should consume a trait.
#[derive(Debug)]
pub enum Object {
  Blob(RawObject),
  Commit(RawObject),
  Tag(RawObject),
  Tree(RawObject),
}

#[derive(Debug)]
pub struct RawObject {
  pub sha:    String,
  pub size:   u32,   // in bytes
  pub offset: usize, // position of first char after null
  reader:     BufReader<ZlibDecoder<File>>,
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

    let object = RawObject {
      sha,
      size,
      reader: zfile,
      offset: buf.len(),
    };

    // let mut content = vec![];
    // zfile.read_to_end(&mut content)?;

    let kind = match string_type {
      "commit" => Self::Commit(object),
      "tag" => Self::Tag(object),
      "tree" => Self::Tree(object),
      "blob" => Self::Blob(object),
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

  pub fn string_type(&self) -> &'static str {
    match self {
      Self::Blob(_) => "blob",
      Self::Commit(_) => "commit",
      Self::Tag(_) => "tag",
      Self::Tree(_) => "tree",
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
