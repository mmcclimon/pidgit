use flate2::bufread::ZlibDecoder;
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
  Blob(u32, Vec<u8>),
  Commit(u32, Vec<u8>),
  Tag(u32, Vec<u8>),
  Tree(u32, Vec<u8>),
}

impl Object {
  pub fn from_path(path: &Path) -> Result<Self> {
    if !path.is_file() {
      let hunks = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();

      let l = hunks.len();
      let sha = format!("{}{}", hunks[l - 2], hunks[l - 1]);
      return Err(PidgitError::ObjectNotFound(sha));
    }

    let f = File::open(path)?;
    let mut zfile = BufReader::new(ZlibDecoder::new(BufReader::new(f)));

    let mut buf = vec![];
    zfile.read_until(b'\0', &mut buf)?;

    // ignore null terminator
    let s = std::str::from_utf8(&buf[0..buf.len() - 1])?;
    let bits = s.split(" ").collect::<Vec<_>>();

    let string_type = bits[0];
    let size: u32 = bits[1].parse()?;

    let mut content = vec![];
    zfile.read_to_end(&mut content)?;

    let kind = match string_type {
      "commit" => Self::Commit(size, content),
      "tag" => Self::Tag(size, content),
      "tree" => Self::Tree(size, content),
      "blob" => Self::Blob(size, content),
      _ => unreachable!(),
    };

    Ok(kind)
  }
}
