use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::fs::Metadata;
use std::path::PathBuf;

// use crate::object::Blob;
use crate::index::{Index, IndexEntry};
use crate::prelude::*;

#[derive(Clone)]
pub struct Tree {
  entries:      HashMap<PathBuf, TreeItem>,
  label:        OsString,
  ordered_keys: Vec<PathBuf>,
}

// meh, these names. A tree *item* is an element of a tree, which is either
// another tree or a path. A tree

#[derive(Debug, Clone)]
pub enum TreeItem {
  Tree(Tree),
  Entry(PathEntry),
}

// I would like these to be &str, which I think could work, but I need to work
// out the lifetimes.
#[derive(Debug, Clone)]
pub struct PathEntry {
  path: PathBuf,
  mode: Mode,
  sha:  String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mode {
  Tree,
  Executable,
  Normal,
}

impl fmt::Debug for Tree {
  #[rustfmt::skip]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let e = self.entries().map(|(_, e)| e).collect::<Vec<_>>();

    f.debug_struct("Tree")
      .field("root", &self.label)
      .field("entries", &e)
      .finish()
  }
}

impl GitObject for Tree {
  fn raw_content(&self) -> Vec<u8> {
    self
      .entries()
      .flat_map(|(_, e)| e.as_entry_bytes())
      .collect()
  }

  fn type_str(&self) -> &str {
    "tree"
  }

  fn pretty(&self) -> Vec<u8> {
    self
      .entries()
      .map(|(_, e)| e.pretty())
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
      let mut mode_buf = vec![];
      reader.read_until(b' ', &mut mode_buf).expect(err);
      mode_buf.pop();
      let mode = Mode::from(&mode_buf[0..]);

      let mut filename = vec![];
      reader.read_until(b'\0', &mut filename).expect(err);
      filename.pop();

      let mut sha = [0u8; sha1::DIGEST_LENGTH];
      reader.read_exact(&mut sha).expect(err);

      let p = OsString::from_vec(filename);

      entries.push(PathEntry {
        mode,
        sha: hex::encode(sha),
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

  pub fn entries(&self) -> EntryIterator {
    EntryIterator {
      tree: &self,
      idx:  0,
    }
  }
}

pub struct EntryIterator<'t> {
  tree: &'t Tree,
  idx:  usize,
}

impl<'t> Iterator for EntryIterator<'t> {
  type Item = (&'t PathBuf, &'t TreeItem);
  fn next(&mut self) -> Option<Self::Item> {
    if self.idx >= self.tree.ordered_keys.len() {
      return None;
    }

    let key = &self.tree.ordered_keys[self.idx];
    self.idx += 1;
    Some((key, self.tree.entries.get(key).unwrap()))
  }
}

impl PathEntry {
  pub fn as_entry_bytes(&self) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    let mut ret = format!("{} ", self.mode.short()).as_bytes().to_vec();

    ret.extend(self.path.file_name().unwrap().as_bytes());
    ret.push(0);
    ret.extend(hex::decode(&self.sha).unwrap());
    ret
  }

  pub fn from_path(path: &PathBuf) -> Result<Self> {
    let meta = path.metadata()?;
    let mode = Mode::from(&meta);
    let sha = util::compute_sha_for_path(path, Some(&meta))?;

    Ok(PathEntry {
      path: path.clone(),
      mode,
      sha: sha.hexdigest(),
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

  pub fn mode(&self) -> &Mode {
    &self.mode
  }

  pub fn is_tree(&self) -> bool {
    self.mode == Mode::Tree
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
      mode: Mode::from(entry.mode()),
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
        Mode::Tree.long(),
        "tree",
        tree.sha().hexdigest(),
        PathBuf::from(&tree.label).display(),
      ),
      TreeItem::Entry(e) => format!(
        "{} {} {}    {}",
        e.mode.long(),
        "blob",
        e.sha,
        PathBuf::from(e.path.file_name().unwrap()).display(),
      ),
    }
  }
}

impl Mode {
  pub fn short(&self) -> &'static str {
    match self {
      Self::Tree => "40000",
      Self::Normal => "100644",
      Self::Executable => "100755",
    }
  }

  pub fn long(&self) -> &'static str {
    match self {
      Self::Tree => "040000",
      _ => self.short(),
    }
  }
}

impl From<&Metadata> for Mode {
  fn from(stat: &Metadata) -> Self {
    use std::os::unix::fs::PermissionsExt;

    if stat.is_dir() {
      return Self::Tree;
    }

    if stat.permissions().mode() & 0o111 != 0 {
      Self::Executable
    } else {
      Self::Normal
    }
  }
}

impl From<&[u8]> for Mode {
  fn from(bytes: &[u8]) -> Self {
    match bytes {
      b"40000" => Self::Tree,
      b"100644" => Self::Normal,
      b"100755" => Self::Executable,
      _ => panic!("unknown mode {:?}", bytes),
    }
  }
}

impl From<u32> for Mode {
  fn from(mode: u32) -> Self {
    let mode_str = format!("{:0>6o}", mode);
    match mode_str.as_str() {
      "040000" => Self::Tree,
      "100644" => Self::Normal,
      "100755" => Self::Executable,
      _ => panic!("unknown mode {:?}", mode_str),
    }
  }
}

impl PartialEq<u32> for Mode {
  fn eq(&self, other: &u32) -> bool {
    self == &Self::from(*other)
  }
}

// this feels wrong to me
impl PartialEq<u32> for &Mode {
  fn eq(&self, other: &u32) -> bool {
    self == &&Mode::from(*other)
  }
}
