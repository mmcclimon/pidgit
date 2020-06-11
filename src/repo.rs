use flate2::{write::ZlibEncoder, Compression};
use sha1::Sha1;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::index::Index;
use crate::object::{Object, Tree};
use crate::prelude::*;

const GIT_DIR_NAME: &'static str = ".pidgit";

#[derive(Debug)]
pub struct Repository {
  work_tree: PathBuf,
  git_dir:   PathBuf,
  ignore:    HashSet<OsString>,
  ftignore:  HashSet<OsString>,
}

// TODO for later
fn default_ignore() -> HashSet<OsString> {
  let mut ignore: HashSet<OsString> = HashSet::new();
  ignore.insert(".git".into());
  ignore.insert(".pidgit".into());
  ignore.insert("target".into());
  ignore.insert(".DS_Store".into());
  ignore
}

fn default_ftignore() -> HashSet<OsString> {
  let mut ftignore: HashSet<OsString> = HashSet::new();
  ftignore.insert("swp".into());
  ftignore.insert("swo".into());
  ftignore
}

impl Repository {
  pub fn from_work_tree(dir: &Path) -> Result<Self> {
    if !dir.is_dir() {
      return Err(PidgitError::Generic(format!(
        "cannot instantiate repo from working tree: {} is not a directory",
        dir.display()
      )));
    }

    Ok(Repository {
      work_tree: dir.to_path_buf(),
      git_dir:   dir.join(GIT_DIR_NAME),
      ignore:    default_ignore(),
      ftignore:  default_ftignore(),
    })
  }

  pub fn from_git_dir(git_dir: &Path) -> Result<Self> {
    let path = git_dir.canonicalize()?;

    let parent = path.parent().ok_or_else(|| {
      PidgitError::Generic(format!(
        "cannot resolve git_dir: {} has no parent",
        path.display()
      ))
    })?;

    Ok(Repository {
      work_tree: parent.to_path_buf(),
      git_dir:   git_dir.to_path_buf(),
      ignore:    default_ignore(),
      ftignore:  default_ftignore(),
    })
  }

  pub fn create_empty(root: &Path) -> Result<Self> {
    let git_dir = root.join(GIT_DIR_NAME);
    DirBuilder::new().create(&git_dir)?;

    Ok(Repository {
      work_tree: root.to_path_buf(),
      git_dir,
      ignore: default_ignore(),
      ftignore: default_ftignore(),
    })
  }

  pub fn work_tree(&self) -> &PathBuf {
    &self.work_tree
  }

  pub fn git_dir(&self) -> &PathBuf {
    &self.git_dir
  }

  // given a path relative to git_dir, create that file
  pub fn create_file<P>(&self, path: P) -> Result<File>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    File::create(self.git_dir.join(path)).map_err(|e| e.into())
  }

  // given a path relative to git_dir, create that file
  pub fn create_dir<P>(&self, path: P) -> Result<()>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    DirBuilder::new()
      .recursive(true)
      .create(self.git_dir.join(path))
      .map_err(|e| e.into())
  }

  // give it a path relative to .git_dir, read into a string
  pub fn read_file<P>(&self, path: P) -> Result<String>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    let mut s = String::new();
    File::open(self.git_dir().join(path))?.read_to_string(&mut s)?;
    Ok(s.trim().to_string())
  }

  fn path_exists<P>(&self, path: P) -> bool
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    self.git_dir().join(path).is_file()
  }

  pub fn object_for_sha(&self, sha: &str) -> Result<Object> {
    Object::from_git_db(&self.path_for_sha(sha))
  }

  // NB returns an absolute path!
  pub fn path_for_sha(&self, sha: &str) -> PathBuf {
    let (first, rest) = sha.split_at(2);
    self.git_dir.join(format!("objects/{}/{}", first, rest))
  }

  pub fn write_object(&self, obj: &dyn GitObject) -> Result<()> {
    let path = self.path_for_sha(&obj.sha().hexdigest());

    // I am ignoring, here, the possibility that this path exists and might
    // somehow conflict??
    if path.is_file() {
      return Ok(());
    }

    // create parent dir!
    std::fs::create_dir_all(path.parent().unwrap())?;

    let file = File::create(&path)
      .expect(&format!("error creating path {}", path.display()));

    let mut e = ZlibEncoder::new(file, Compression::default());

    e.write_all(&obj.header())?;
    e.write_all(&obj.raw_content())?;
    e.finish()?;

    Ok(())
  }

  pub fn resolve_object(&self, name: &str) -> Result<Object> {
    // this may get more smarts later
    let to_match = match name {
      "head" | "@" => "HEAD",
      _ => name,
    };

    // this algorithm directly from git rev-parse docs
    for prefix in &[".", "refs", "refs/tags", "refs/heads", "refs/remotes"] {
      let joined = format!("{}/{}", prefix, to_match);

      if self.path_exists(&joined) {
        return self.resolve_ref(&joined);
      }
    }

    // also check head of remotes
    let remote_head = format!("refs/remotes/{}/HEAD", to_match);
    if self.path_exists(&remote_head) {
      return self.resolve_ref(&remote_head);
    }

    // not found yet, assume a sha
    self.resolve_sha(to_match)
  }

  fn resolve_ref(&self, refstr: &str) -> Result<Object> {
    let raw = self.read_file(refstr)?;

    if raw.starts_with("ref: ") {
      let symref = raw.trim_start_matches("ref: ");
      self.resolve_ref(symref)
    } else {
      self.object_for_sha(&raw)
    }
  }

  fn resolve_sha(&self, sha: &str) -> Result<Object> {
    if sha.len() < 4 {
      return Err(PidgitError::ObjectNotFound(format!(
        "{} is too short to be a sha",
        sha
      )));
    }

    if sha.len() == 40 {
      return self.object_for_sha(sha);
    }

    let not_found = Err(PidgitError::ObjectNotFound(sha.to_string()));

    // we need to walk the objects dir
    let (prefix, rest) = sha.split_at(2);
    let base = self.git_dir.join(format!("objects/{}", &prefix));

    if !base.is_dir() {
      return not_found;
    }

    let paths = std::fs::read_dir(base)?
      .filter(|e| e.is_ok())
      .map(|e| e.unwrap().path())
      .filter(|path| {
        path
          .file_name()
          .unwrap()
          .to_string_lossy()
          .starts_with(&rest)
      })
      .collect::<Vec<_>>();

    match paths.len() {
      0 => not_found,
      1 => Object::from_git_db(&paths[0]),
      _ => Err(PidgitError::ObjectNotFound(format!("{} is ambiguous", sha))),
    }
  }

  pub fn index(&self) -> Result<Index> {
    Index::from_path(self.git_dir().join("index"))
  }

  pub fn write_index(&self, index: &Index) -> Result<()> {
    index.write(self.git_dir().join("index"))
  }

  pub fn as_tree(&self) -> Result<Tree> {
    use crate::object::PathEntry;

    let entries = self
      .list_files()?
      .iter()
      .filter_map(|entry| PathEntry::from_path(&entry).ok())
      .collect::<Vec<_>>();

    Ok(Tree::build(entries))
  }

  pub fn write_tree(&self, tree: &Tree) -> Result<()> {
    tree.traverse(&|t| self.write_object(t))
  }

  pub fn update_head(&self, new_sha: &Sha1) -> Result<()> {
    use std::fs::OpenOptions;

    let raw = self.read_file("HEAD")?;

    let suffix = if raw.starts_with("ref: ") {
      raw.trim_start_matches("ref: ")
    } else {
      // must be a sha
      "HEAD"
    };

    let mut f = OpenOptions::new()
      .write(true)
      .open(self.git_dir().join(suffix))?;

    f.write_all(format!("{}\n", new_sha.hexdigest()).as_bytes())?;
    Ok(())
  }

  pub fn list_files(&self) -> Result<Vec<PathBuf>> {
    self.list_files_from_base(&self.work_tree)
  }

  pub fn list_files_from_base(&self, base: &PathBuf) -> Result<Vec<PathBuf>> {
    if !base.exists() {
      return Err(PidgitError::PathspecNotFound(
        base.as_os_str().to_os_string(),
      ));
    }

    let mut dir_entries = vec![];

    let relativize =
      |p: &PathBuf| p.strip_prefix(&self.work_tree).unwrap().to_path_buf();

    let abs_base = base.canonicalize()?;

    if abs_base.is_file() {
      dir_entries.push(relativize(&abs_base));
      return Ok(dir_entries);
    }

    for e in std::fs::read_dir(abs_base)?.filter_map(std::result::Result::ok) {
      let path = e.path();

      if self.ignore.contains(path.file_name().unwrap()) {
        continue;
      }

      if let Some(ext) = path.extension() {
        if self.ftignore.contains(ext) {
          continue;
        }
      }

      if path.is_dir() {
        dir_entries.extend(self.list_files_from_base(&path)?);
      } else {
        dir_entries.push(relativize(&path));
      }
    }

    dir_entries.sort_unstable_by(|a, b| {
      format!("{}", a.display()).cmp(&format!("{}", b.display()))
    });

    Ok(dir_entries)
  }
}
