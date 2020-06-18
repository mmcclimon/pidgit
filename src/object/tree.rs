use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

// use crate::object::Blob;
use crate::index::{Index, IndexEntry};
use crate::prelude::*;

pub struct Tree {
  entries:      HashMap<PathBuf, TreeItem>,
  label:        OsString,
  ordered_keys: Vec<PathBuf>,
}

// meh, these names. A tree *item* is an element of a tree, which is either
// another tree or a path. A tree

#[derive(Debug)]
pub enum TreeItem {
  Tree(Tree),
  Entry(PathEntry),
}

// I would like these to be &str, which I think could work, but I need to work
// out the lifetimes.
#[derive(Debug)]
pub struct PathEntry {
  path: PathBuf,
  mode: String,
  sha:  String,
}

impl fmt::Debug for Tree {
  #[rustfmt::skip]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let e = self
      .ordered_keys
      .iter()
      .map(|k| self.entries.get(k).unwrap())
      .collect::<Vec<_>>();

    f.debug_struct("Tree")
      .field("root", &self.label)
      .field("entries", &e)
      .finish()
  }
}

impl GitObject for Tree {
  fn raw_content(&self) -> Vec<u8> {
    self
      .ordered_keys
      .iter()
      .flat_map(|k| self.entries.get(k).unwrap().as_entry_bytes())
      .collect()
  }

  fn type_str(&self) -> &str {
    "tree"
  }

  fn pretty(&self) -> Vec<u8> {
    self
      .ordered_keys
      .iter()
      .map(|key| self.entries.get(key).unwrap().pretty())
      .collect::<Vec<_>>()
      .join("\n")
      .as_bytes()
      .to_vec()
  }
}

impl Tree {
  pub fn new(label: OsString) -> Self {
    Self {
      entries: HashMap::new(),
      label,
      ordered_keys: vec![],
    }
  }

  pub fn from_content(content: Vec<u8>) -> Self {
    use std::io::prelude::*;
    use std::io::Cursor;
    use std::os::unix::ffi::OsStringExt;

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

      let p = OsString::from_vec(filename);

      entries.push(PathEntry {
        mode: mode_str,
        sha:  hex::encode(sha),
        path: p.into(),
      });
    }

    Self::build(entries)
  }

  // assumes entries are correctly sorted!
  pub fn build(entries: Vec<PathEntry>) -> Self {
    let mut root = Tree::new("".into());

    for entry in entries {
      let parents = entry.parents();
      root.add_entry(&parents, entry);
    }

    root
  }

  pub fn add_entry(&mut self, parents: &[PathBuf], entry: PathEntry) {
    if parents.is_empty() {
      // let basename = entry.path.file_name().unwrap().to_os_string();
      self.ordered_keys.push(entry.path.clone());
      self
        .entries
        .insert(entry.path.clone(), TreeItem::Entry(entry));
      return;
    }

    // recurse
    let key = parents[0].clone();

    if !self.entries.contains_key(&key) {
      let label = OsString::from(key.file_name().unwrap());
      self.ordered_keys.push(key.clone());
      self
        .entries
        .insert(key.clone(), TreeItem::Tree(Tree::new(label)));
    }

    if let TreeItem::Tree(tree) = self.entries.get_mut(&key).unwrap() {
      tree.add_entry(&parents[1..], entry);
    }
  }

  pub fn traverse<F>(&self, f: &F) -> Result<()>
  where
    F: Fn(&Tree) -> Result<()>,
  {
    for item in self.entries.values() {
      if let TreeItem::Tree(tree) = item {
        tree.traverse(f)?;
      }
    }

    f(self)
  }

  pub fn as_entry_bytes(&self) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    let mut ret = "40000 ".as_bytes().to_vec();
    ret.extend(self.label.as_bytes());
    ret.push(0);
    ret.extend(self.sha().digest().bytes().iter());
    ret
  }
}

impl PathEntry {
  pub fn as_entry_bytes(&self) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    let mut ret = format!("{} ", self.mode.trim_start_matches("0"),)
      .as_bytes()
      .to_vec();

    ret.extend(self.path.file_name().unwrap().as_bytes());
    ret.push(0);
    ret.extend(hex::decode(&self.sha).unwrap());
    ret
  }

  pub fn from_path(path: &PathBuf) -> Result<Self> {
    use std::os::unix::fs::PermissionsExt;

    let perms = path.metadata()?.permissions();

    let mode = if perms.mode() & 0o111 != 0 {
      "100755"
    } else {
      "100644"
    };

    let sha = util::compute_sha_for_path(path)?;

    Ok(PathEntry {
      path: path.clone(),
      mode: mode.to_string(),
      sha:  sha.hexdigest(),
    })
  }

  pub fn parents(&self) -> Vec<PathBuf> {
    let mut parents = self
      .path
      .ancestors()
      .skip(1)
      .map(|p| p.to_path_buf())
      .collect::<Vec<_>>();

    parents.pop(); // remove empty path
    parents.reverse();

    parents
  }

  // NB this is a string, not a Sha1!
  pub fn sha(&self) -> &str {
    &self.sha
  }
}

impl From<&Index> for Tree {
  fn from(idx: &Index) -> Self {
    let entries = idx.entries().map(|e| e.into()).collect::<Vec<PathEntry>>();
    Self::build(entries)
  }
}

impl PartialOrd for PathEntry {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    // git compares files a little weirdly, so we must coerce to strings
    format!("{}", self.path.display())
      .partial_cmp(&format!("{}", other.path.display()))
  }
}

impl Ord for PathEntry {
  fn cmp(&self, other: &Self) -> Ordering {
    format!("{}", self.path.display()).cmp(&format!("{}", other.path.display()))
  }
}

impl Eq for PathEntry {}
impl PartialEq for PathEntry {
  fn eq(&self, other: &Self) -> bool {
    self.path == other.path && self.mode == other.mode
  }
}

impl From<&IndexEntry> for PathEntry {
  fn from(entry: &IndexEntry) -> Self {
    Self {
      mode: format!("{:0>6o}", entry.mode()),
      path: PathBuf::from(entry.name.clone()),
      sha:  entry.sha.clone(),
    }
  }
}

impl TreeItem {
  pub fn as_entry_bytes(&self) -> Vec<u8> {
    match self {
      TreeItem::Tree(t) => t.as_entry_bytes(),
      TreeItem::Entry(e) => e.as_entry_bytes(),
    }
  }

  pub fn pretty(&self) -> String {
    match self {
      TreeItem::Tree(tree) => format!(
        "{} {} {}    {}",
        "040000",
        "tree",
        tree.sha().hexdigest(),
        PathBuf::from(&tree.label).display(),
      ),
      TreeItem::Entry(e) => format!(
        "{} {} {}    {}",
        e.mode,
        "blob",
        e.sha,
        PathBuf::from(e.path.file_name().unwrap()).display(),
      ),
    }
  }
}
