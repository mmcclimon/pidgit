use crate::object::{GitObject, RawObject};
use crate::Result;

#[derive(Debug)]
pub struct Tree {
  raw:     RawObject,
  content: Vec<u8>,
}

impl GitObject for Tree {
  fn get_ref(&self) -> &RawObject {
    &self.raw
  }

  fn pretty(&self) -> Result<String> {
    use std::io::prelude::*;
    use std::io::Cursor;

    // a tree is made of entries, where each entry entry is:
    // mode filename NULL 20-bytes-of-sha
    let mut reader = Cursor::new(&self.raw.content);
    let len = reader.get_ref().len();

    let mut out = String::new();

    while (reader.position() as usize) < len {
      let mut mode = vec![];
      reader.read_until(b' ', &mut mode)?;
      mode.pop();

      let mut filename = vec![];
      reader.read_until(b'\0', &mut filename)?;
      filename.pop();

      let mut sha = [0u8; 20];
      reader.read_exact(&mut sha)?;

      let mode_str = format!("{:0>6}", String::from_utf8(mode)?);

      let entry_type = match &mode_str[..3] {
        "040" => "tree",
        "100" => "blob",
        "120" => "blob", // symlink
        _ => "????",
      };

      out.push_str(&format!(
        "{} {} {}    {}\n",
        mode_str,
        entry_type,
        hex::encode(sha),
        String::from_utf8(filename)?,
      ));
    }

    out.pop();

    return Ok(out);
  }
}

impl Tree {
  pub fn from_raw(raw: RawObject) -> Self {
    Self {
      raw,
      content: vec![],
    }
  }
}
