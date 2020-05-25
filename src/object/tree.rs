use crate::object::GitObject;

#[derive(Debug)]
pub struct Tree {
  entries: Vec<TreeEntry>,
}

// I would like these to be &str, which I think could work, but I need to work
// out the lifetimes.
#[derive(Debug)]
pub struct TreeEntry {
  mode: String,
  name: String,
  sha:  String,
  kind: String,
}

impl GitObject for Tree {
  fn raw_content(&self) -> &Vec<u8> {
    todo!()
  }

  fn type_str(&self) -> &str {
    "tree"
  }
}

/*
  fn pretty(&self) -> Vec<u8> {
    self
      .entries
      .iter()
      .map(|e| format!("{} {} {}    {}", e.mode, e.kind, e.sha, e.name,))
      .collect::<Vec<_>>()
      .join("\n")
      .as_bytes()
      .to_vec()
  }
*/

impl Tree {
  pub fn from_content(content: Vec<u8>) -> Self {
    use std::io::prelude::*;
    use std::io::Cursor;

    let err = "malformed tree entry";

    // a tree is made of entries, where each entry entry is:
    // mode filename NULL 20-bytes-of-sha
    let mut entries = vec![];

    let mut reader = Cursor::new(&content);
    let len = reader.get_ref().len();

    while (reader.position() as usize) < len {
      let mut mode = vec![];
      reader.read_until(b' ', &mut mode).expect(err);
      mode.pop();

      let mut filename = vec![];
      reader.read_until(b'\0', &mut filename).expect(err);
      filename.pop();

      let mut sha = [0u8; sha1::DIGEST_LENGTH];
      reader.read_exact(&mut sha).expect(err);

      let mode_str = format!("{:0>6}", String::from_utf8(mode).expect(err));

      let entry_type = match &mode_str[..3] {
        "040" => "tree",
        "100" => "blob",
        "120" => "blob", // symlink
        _ => "????",
      };

      entries.push(TreeEntry {
        mode: mode_str,
        kind: entry_type.to_string(),
        name: String::from_utf8_lossy(&filename).to_string(), // improve me
        sha:  hex::encode(sha),
      });
    }

    Self { entries }
  }
}
