use crate::object::{Blob, GitObject, Result};
use std::io::prelude::*;
use std::path::PathBuf;

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
  fn raw_content(&self) -> Vec<u8> {
    self.entries.iter().flat_map(|e| e.as_bytes()).collect()
  }

  fn type_str(&self) -> &str {
    "tree"
  }

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
}

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

  pub fn from_path(base: &PathBuf) -> Result<Self> {
    // hard-coding the ignores for now...
    use std::collections::HashSet;
    use std::ffi::OsString;
    let mut ignore: HashSet<OsString> = HashSet::new();
    ignore.insert(".git".into());
    ignore.insert(".pidgit".into());
    ignore.insert("target".into());
    ignore.insert(".DS_Store".into());

    let mut ftignore: HashSet<OsString> = HashSet::new();
    ftignore.insert("swp".into());
    ftignore.insert("swo".into());

    let mut dir_entries = std::fs::read_dir(base)?
      .filter_map(std::result::Result::ok)
      .map(|e| e.path())
      .collect::<Vec<_>>();

    dir_entries.sort();

    let mut entries = vec![];

    for path in dir_entries {
      if ignore.contains(path.file_name().unwrap()) {
        continue;
      }

      if let Some(ext) = path.extension() {
        if ftignore.contains(ext) {
          continue;
        }
      }

      let e = Self::entry_for_path(&path)?;
      entries.push(e);
    }

    // println!("{:?}", entries);

    Ok(Self { entries })
  }

  fn entry_for_path(path: &PathBuf) -> Result<TreeEntry> {
    if path.is_dir() {
      let tree = Self::from_path(&path)?;
      Ok(TreeEntry {
        mode: "040000".to_string(), // todo
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        sha:  tree.sha().hexdigest(),
        kind: tree.type_str().to_string(),
      })
    } else {
      TreeEntry::from_path(&path)
    }
  }
}

impl TreeEntry {
  pub fn as_bytes(&self) -> Vec<u8> {
    let mut ret =
      format!("{} {}\0", self.mode.trim_start_matches("0"), self.name)
        .as_bytes()
        .to_vec();
    ret.extend(hex::decode(&self.sha).unwrap());
    ret
  }

  // XXX poorly named: only works on blobs
  pub fn from_path(path: &PathBuf) -> Result<Self> {
    use std::fs::File;
    use std::io::BufReader;

    let mut content = vec![];
    let mut reader = BufReader::new(File::open(&path)?);
    reader.read_to_end(&mut content)?;

    let blob = Blob::from_content(content);

    Ok(TreeEntry {
      mode: "100644".to_string(), // todo
      name: path.file_name().unwrap().to_string_lossy().to_string(),
      sha:  blob.sha().hexdigest(),
      kind: "blob".to_string(),
    })
  }
}
