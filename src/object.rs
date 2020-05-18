use flate2::bufread::ZlibDecoder;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::Result;

// object is a pretty generic name, but hey
#[derive(Debug)]
pub enum Object {
  Generic,
  Blob(u32),
  Commit(u32),
  Tag(u32),
  Tree(u32),
  NotFound,
}

impl Object {
  pub fn from_path(path: &Path) -> Result<Self> {
    if !path.is_file() {
      return Ok(Self::NotFound);
    }

    let f = File::open(path)?;
    let mut zfile = BufReader::new(ZlibDecoder::new(BufReader::new(f)));

    let mut buf = vec![];
    let num_bytes = zfile.read_until(b'\0', &mut buf)?;

    if num_bytes == 0 {
      eprintln!("could not read bytes from file? {}", path.display());
      return Ok(Self::NotFound);
    }

    // ignore null terminator
    let s = std::str::from_utf8(&buf[0..buf.len() - 1])?;
    let bits = s.split(" ").collect::<Vec<_>>();

    let string_type = bits[0];
    let size: u32 = bits[1].parse()?;

    let kind = match string_type {
      "commit" => Self::Commit(size),
      "tag" => Self::Tag(size),
      "tree" => Self::Tree(size),
      "blob" => Self::Blob(size),
      _ => {
        eprintln!("weird object type! {}", bits[0]);
        Self::Generic
      },
    };

    Ok(kind)
  }
}
