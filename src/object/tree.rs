use sha1::Sha1;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::prelude::*;
use std::path::PathBuf;

// use crate::object::Blob;
use crate::prelude::*;

#[derive(Debug)]
pub struct Tree {
  entries: BTreeMap<PathBuf, TreeItem>,
  label:   String,
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

impl GitObject for Tree {
  fn raw_content(&self) -> Vec<u8> {
    self
      .entries
      .values()
      .flat_map(|e| e.as_entry_bytes())
      .collect()
  }

  fn type_str(&self) -> &str {
    "tree"
  }

  fn pretty(&self) -> Vec<u8> {
    self
      .entries
      .iter()
      .map(|(path, item)| match item {
        TreeItem::Tree(tree) => {
          let name = path.file_name().unwrap().to_string_lossy();
          format!(
            "{} {} {}    {}",
            "040000",
            "tree",
            tree.sha().hexdigest(),
            name,
          )
        },
        TreeItem::Entry(e) => format!(
          "{} {} {}    {}",
          e.mode,
          "blob",
          e.sha,
          e.path.file_name().unwrap().to_string_lossy(),
        ),
      })
      .collect::<Vec<_>>()
      .join("\n")
      .as_bytes()
      .to_vec()
  }
}

impl Tree {
  pub fn new(label: String) -> Self {
    Self {
      entries: BTreeMap::new(),
      label,
    }
  }

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

      // let entry_type = match &mode_str[..3] {
      //   "040" => "tree",
      //   "100" => "blob",
      //   "120" => "blob", // symlink
      //   _ => "????",
      // };

      let p = String::from_utf8_lossy(&filename);

      entries.push(PathEntry {
        mode: mode_str,
        // kind: entry_type.to_string(),
        // name: String::from_utf8_lossy(&filename).to_string(), // improve me
        sha:  hex::encode(sha),
        path: PathBuf::from(p.to_string()),
      });
    }

    // Self { entries }
    todo!("implement this correctly");
  }

  pub fn from_path(base: &PathBuf) -> Result<Self> {
    // hard-coding the ignores for now...
    use std::collections::HashSet;
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

    // Ok(Self { entries })
    todo!("account for hashmap entries")
  }

  pub fn build(entries: Vec<PathEntry>) -> Self {
    let mut root = Tree::new("".to_string());

    for entry in entries {
      let parents = entry.parents();
      root.add_entry(&parents, entry);
    }

    root
  }

  pub fn add_entry(&mut self, parents: &[PathBuf], entry: PathEntry) {
    if parents.is_empty() {
      // let basename = entry.path.file_name().unwrap().to_os_string();
      self
        .entries
        .insert(entry.path.clone(), TreeItem::Entry(entry));
      return;
    }

    // recurse
    let key = parents[0].clone();

    if !self.entries.contains_key(&key) {
      let label = key.file_name().unwrap().to_string_lossy();
      self
        .entries
        .insert(key.clone(), TreeItem::Tree(Tree::new(label.into_owned())));
    }

    if let TreeItem::Tree(tree) = self.entries.get_mut(&key).unwrap() {
      tree.add_entry(&parents[1..], entry);
    }
  }

  pub fn traverse<F>(&self, f: &F)
  where
    F: Fn(&Tree),
  {
    for item in self.entries.values() {
      if let TreeItem::Tree(tree) = item {
        tree.traverse(f);
      }
    }

    f(self)
  }

  pub fn as_entry_bytes(&self) -> Vec<u8> {
    let mut ret = format!("40000 {}\0", self.label).as_bytes().to_vec();
    ret.extend(self.sha().digest().bytes().iter());
    ret
  }

  fn entry_for_path(path: &PathBuf) -> Result<PathEntry> {
    if path.is_dir() {
      let tree = Self::from_path(&path)?;
      Ok(PathEntry {
        mode: "040000".to_string(), // todo
        // name: path.file_name().unwrap().to_string_lossy().to_string(),
        sha:  tree.sha().hexdigest(),
        // kind: tree.type_str().to_string(),
        path: path.clone(),
      })
    } else {
      PathEntry::from_path(&path)
    }
  }

  pub fn write(&self, _repo: &Repository) -> Result<()> {
    todo!("re-implement with new entry abstraction");

    /*
    for e in &self.entries {
      let git_path = repo.path_for_sha(&e.sha);
      if git_path.is_file() {
        // println!("have {} {}", e.kind, e.name);
        continue;
      }


      if let Some(ref path) = e.path {
        match e.kind.as_str() {
          "blob" => {
            // println!("need to write blob {}", e.name);
            let blob = Blob::from_path(path)?;
            repo.write_object(&blob)?;
          },
          "tree" => {
            // println!("need to write tree: {}", e.name);
            let t = Self::from_path(path)?;
            t.write(repo)?;
          },
          _ => panic!("unknown type!"),
        }
      } else {
        return Err(PidgitError::Generic(
          "cannot recurse on PathEntry with no path".to_string(),
        ));
      }
    }

    repo.write_object(self)
    */
  }
}

impl PathEntry {
  pub fn as_entry_bytes(&self) -> Vec<u8> {
    let mut ret = format!(
      "{} {}\0",
      self.mode.trim_start_matches("0"),
      self.path.file_name().unwrap().to_string_lossy(),
    )
    .as_bytes()
    .to_vec();
    ret.extend(hex::decode(&self.sha).unwrap());
    ret
  }

  // XXX poorly named: only works on blobs
  pub fn from_path(path: &PathBuf) -> Result<Self> {
    use std::fs::File;
    use std::io::BufReader;
    use std::os::unix::fs::PermissionsExt;

    let meta = path.metadata()?;
    let perms = meta.permissions();

    let mode = if perms.mode() & 0o111 != 0 {
      "100755"
    } else {
      "100644"
    };

    // read this file, but don't slurp the whole thing into memory
    let mut reader = BufReader::new(File::open(&path)?);
    let mut sha = Sha1::new();

    sha.update(format!("blob {}\0", meta.len()).as_bytes());

    loop {
      let buf = reader.fill_buf()?;
      let len = buf.len();

      // EOF
      if len == 0 {
        break;
      }

      sha.update(&buf);
      reader.consume(len);
    }

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

impl PartialOrd for PathEntry {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.path.partial_cmp(&other.path)
  }
}

impl Ord for PathEntry {
  fn cmp(&self, other: &Self) -> Ordering {
    self.path.cmp(&other.path)
  }
}

impl Eq for PathEntry {}
impl PartialEq for PathEntry {
  fn eq(&self, other: &Self) -> bool {
    self.path == other.path && self.mode == other.mode
  }
}

impl TreeItem {
  pub fn as_entry_bytes(&self) -> Vec<u8> {
    match self {
      TreeItem::Tree(t) => t.as_entry_bytes(),
      TreeItem::Entry(e) => e.as_entry_bytes(),
    }
  }
}
